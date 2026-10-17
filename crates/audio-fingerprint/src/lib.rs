//! Embedded, read-only Chromaprint/FFmpeg fingerprint production.
//!
//! Native cancellation is cooperative. A timed-out caller must retain/drop its
//! running worker normally; its admission permit is released only on return.
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Busy,
    Cancelled,
    TimedOut,
    NotRegularFile,
    TooShort,
    InvalidDuration,
    Io(String),
    Decode(String),
    Internal(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => {
                f.write_str("A previous fingerprint is still stopping. Try again shortly.")
            }
            Self::Cancelled => f.write_str("Fingerprinting was cancelled."),
            Self::TimedOut => f.write_str("Fingerprinting reached its time limit."),
            Self::NotRegularFile => f.write_str("Fingerprinting requires a local audio file."),
            Self::TooShort => f.write_str("There is not enough audio to calculate a fingerprint."),
            Self::InvalidDuration => f.write_str("The full audio duration is unavailable."),
            Self::Io(value) => write!(f, "The audio file could not be read: {value}"),
            Self::Decode(value) => write!(f, "The audio could not be decoded: {value}"),
            Self::Internal(value) => write!(f, "The audio could not be fingerprinted: {value}"),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Clone)]
pub struct Control {
    timeout: Duration,
    cancelled: Arc<dyn Fn() -> bool + Send + Sync>,
}
impl Control {
    pub fn new(timeout: Duration, cancelled: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            timeout,
            cancelled: Arc::new(cancelled),
        }
    }
}
impl Default for Control {
    fn default() -> Self {
        Self::new(Duration::from_secs(120), || false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fingerprint {
    /// Full stream duration with fpcalc's centisecond producer quantization.
    pub duration: f64,
    pub encoded: String,
    /// Normalized mono frames processed, capped at 120 * 11,025.
    pub processed_frames: u64,
}

#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub chromaprint: String,
    pub ffmpeg: String,
    pub license: String,
    pub audio_decoders: u32,
    pub non_audio_decoders: u32,
    pub protocols: u32,
}
pub fn backend_info() -> BackendInfo {
    let mut native = NativeInfo {
        chromaprint: std::ptr::null(),
        ffmpeg: std::ptr::null(),
        license: std::ptr::null(),
        audio_decoders: 0,
        non_audio_decoders: 0,
        protocols: 0,
    };
    // SAFETY: the native function writes only this struct, returns static
    // NUL-terminated version/license strings, and retains no Rust pointers.
    unsafe { opentag_audio_info(&mut native) };
    let text = |pointer: *const c_char| {
        if pointer.is_null() {
            return String::new();
        }
        // SAFETY: strings remain owned by the linked libraries for process life.
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned()
    };
    BackendInfo {
        chromaprint: text(native.chromaprint),
        ffmpeg: text(native.ffmpeg),
        license: text(native.license),
        audio_decoders: native.audio_decoders,
        non_audio_decoders: native.non_audio_decoders,
        protocols: native.protocols,
    }
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self, Error> {
        ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| Error::Busy)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}

/// Reserve admission before spawning a blocking task, so repeated searches
/// cannot queue unbounded native jobs behind a stalled read or decoder.
pub struct Job {
    path: PathBuf,
    control: Control,
    deadline: Instant,
    _permit: Permit,
}
impl Job {
    pub fn new(path: PathBuf, control: Control) -> Result<Self, Error> {
        let permit = Permit::acquire()?;
        let deadline = Instant::now()
            .checked_add(control.timeout)
            .ok_or_else(|| Error::Internal("Invalid fingerprint time budget".into()))?;
        Ok(Self {
            path,
            control,
            deadline,
            _permit: permit,
        })
    }

    /// Run only in a blocking worker. This reads but never writes the file.
    pub fn run(self) -> Result<Fingerprint, Error> {
        let mut context = Context {
            file: None,
            control: self.control.clone(),
            deadline: self.deadline,
            failure: None,
            length: 0,
        };
        context.check()?;
        let metadata = std::fs::metadata(&self.path).map_err(|e| Error::Io(e.to_string()))?;
        if !metadata.is_file() {
            return Err(Error::NotRegularFile);
        }
        let mut options = OpenOptions::new();
        options.read(true);
        // If a file is replaced with a FIFO between metadata/open, avoid a
        // blocking FIFO open and reject it with the descriptor metadata check.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NONBLOCK);
        }
        let file = options
            .open(&self.path)
            .map_err(|e| Error::Io(e.to_string()))?;
        let metadata = file.metadata().map_err(|e| Error::Io(e.to_string()))?;
        if !metadata.is_file() {
            return Err(Error::NotRegularFile);
        }
        context.length = metadata.len();
        context.file = Some(file);
        context.check()?;
        let extension = self
            .path
            .extension()
            .and_then(|s| s.to_str())
            .filter(|s| s.len() <= 16 && s.bytes().all(|c| c.is_ascii_alphanumeric()));
        let hint = CString::new(
            extension
                .map(|s| format!("input.{s}"))
                .unwrap_or("input".into()),
        )
        .map_err(|_| Error::Internal("Invalid audio format hint".into()))?;
        let source = NativeSource {
            user: (&mut context as *mut Context).cast(),
            read: read_callback,
            seek: seek_callback,
            interrupted: interrupt_callback,
            format_hint: hint.as_ptr(),
        };
        let mut native = NativeResult {
            status: 5,
            duration_ms: 0,
            processed_frames: 0,
            fingerprint: std::ptr::null_mut(),
            error: [0; 512],
        };
        // SAFETY: context/hint/source outlive this synchronous call. Native
        // code retains no pointers, invokes callbacks on this thread only,
        // and contains exceptions. Result allocation is freed by ResultGuard.
        let status = unsafe { opentag_audio_fingerprint(&source, &mut native) };
        let guard = ResultGuard(&mut native);
        context.check()?;
        if status != 0 {
            return Err(match status {
                1 => Error::Decode(native_error(&native.error)),
                2 => Error::TooShort,
                3 => Error::InvalidDuration,
                _ => Error::Internal(native_error(&native.error)),
            });
        }
        if native.fingerprint.is_null() {
            return Err(Error::Internal("Missing native fingerprint".into()));
        }
        // SAFETY: successful native output is a NUL-terminated owned buffer;
        // guard keeps it alive until the owned Rust String is copied.
        let encoded = unsafe { CStr::from_ptr(native.fingerprint) }
            .to_str()
            .map_err(|_| Error::Internal("Invalid native fingerprint encoding".into()))?
            .to_owned();
        if encoded.is_empty()
            || !encoded
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(Error::Internal(
                "Invalid native fingerprint encoding".into(),
            ));
        }
        let duration = duration_from_ms(native.duration_ms)?;
        let result = Fingerprint {
            duration,
            encoded,
            processed_frames: native.processed_frames,
        };
        drop(guard);
        Ok(result)
    }
}

fn duration_from_ms(milliseconds: i64) -> Result<f64, Error> {
    if milliseconds <= 0 {
        return Err(Error::InvalidDuration);
    }
    // Locale-independent equivalent of fpcalc's %.2f JSON producer. Rounding
    // raw milliseconds directly for lookup changes 49.499s from 50 to 49.
    format!("{:.2}", milliseconds as f64 / 1000.0)
        .parse()
        .map_err(|_| Error::InvalidDuration)
}

struct Context {
    file: Option<File>,
    control: Control,
    deadline: Instant,
    failure: Option<Error>,
    length: u64,
}
impl Context {
    fn check(&mut self) -> Result<(), Error> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let error = if (self.control.cancelled)() {
            Some(Error::Cancelled)
        } else if Instant::now() >= self.deadline {
            Some(Error::TimedOut)
        } else {
            None
        };
        if let Some(error) = error {
            self.failure = Some(error.clone());
            return Err(error);
        }
        Ok(())
    }
}

fn callback<T>(
    context: &mut Context,
    fallback: T,
    work: impl FnOnce(&mut Context) -> Result<T, Error>,
) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(context))) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            context.failure = Some(error);
            fallback
        }
        Err(_) => {
            context.failure = Some(Error::Internal("Audio source callback failed".into()));
            fallback
        }
    }
}

unsafe extern "C" fn read_callback(user: *mut c_void, data: *mut u8, length: c_int) -> c_int {
    if user.is_null() || data.is_null() || length <= 0 {
        return -1;
    }
    // SAFETY: native supplies the live exclusive Context and writable input
    // buffer, bounded by the positive length. No callback escapes the call.
    let context = unsafe { &mut *user.cast::<Context>() };
    callback(context, -1, |context| {
        context.check()?;
        let buffer = unsafe { std::slice::from_raw_parts_mut(data, length as usize) };
        context
            .file
            .as_mut()
            .ok_or_else(|| Error::Internal("Missing audio file".into()))?
            .read(buffer)
            .map(|n| n as c_int)
            .map_err(|e| Error::Io(e.to_string()))
    })
}

unsafe extern "C" fn seek_callback(user: *mut c_void, offset: i64, whence: c_int) -> i64 {
    if user.is_null() {
        return -1;
    }
    // SAFETY: same synchronous exclusive Context contract as read_callback.
    let context = unsafe { &mut *user.cast::<Context>() };
    callback(context, -1, |context| {
        context.check()?;
        if whence == 0x10000 {
            // FFmpeg AVSEEK_SIZE, a query with no seek.
            return i64::try_from(context.length)
                .map_err(|_| Error::Io("Audio file is too large".into()));
        }
        let from = match whence {
            0 if offset >= 0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => return Err(Error::Io("Invalid audio seek".into())),
        };
        let position = context
            .file
            .as_mut()
            .ok_or_else(|| Error::Internal("Missing audio file".into()))?
            .seek(from)
            .map_err(|e| Error::Io(e.to_string()))?;
        i64::try_from(position).map_err(|_| Error::Io("Audio seek is too large".into()))
    })
}

unsafe extern "C" fn interrupt_callback(user: *mut c_void) -> c_int {
    if user.is_null() {
        return 1;
    }
    // SAFETY: native retains no callback/user pointer after its call returns.
    let context = unsafe { &mut *user.cast::<Context>() };
    callback(context, 1, |context| {
        context.check()?;
        Ok(0)
    })
}

#[repr(C)]
struct NativeSource {
    user: *mut c_void,
    read: unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int,
    seek: unsafe extern "C" fn(*mut c_void, i64, c_int) -> i64,
    interrupted: unsafe extern "C" fn(*mut c_void) -> c_int,
    format_hint: *const c_char,
}
#[repr(C)]
struct NativeResult {
    status: c_int,
    duration_ms: i64,
    processed_frames: u64,
    fingerprint: *mut c_char,
    error: [c_char; 512],
}
struct ResultGuard(*mut NativeResult);
impl Drop for ResultGuard {
    fn drop(&mut self) {
        // SAFETY: the pointer refers to this call's live NativeResult only;
        // the C++ free function accepts null/empty fingerprint allocations.
        unsafe { opentag_audio_result_free(self.0) };
    }
}
fn native_error(buffer: &[c_char; 512]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&b| b != 0)
        .map(|b| b.to_ne_bytes()[0])
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}
extern "C" {
    fn opentag_audio_fingerprint(source: *const NativeSource, result: *mut NativeResult) -> c_int;
    fn opentag_audio_result_free(result: *mut NativeResult);
    fn opentag_audio_info(info: *mut NativeInfo);
}

#[repr(C)]
struct NativeInfo {
    chromaprint: *const c_char,
    ffmpeg: *const c_char,
    license: *const c_char,
    audio_decoders: u32,
    non_audio_decoders: u32,
    protocols: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cli_quantization_precedes_lookup_rounding() {
        for milliseconds in [49_499, 49_500, 49_501] {
            assert_eq!(duration_from_ms(milliseconds).unwrap().round() as u64, 50);
        }
        assert_eq!(duration_from_ms(8495).unwrap(), 8.49);
        assert!(duration_from_ms(0).is_err());
        assert!(duration_from_ms(-1).is_err());
    }
    #[test]
    fn native_error_preserves_utf8_bytes_and_stops_at_nul() {
        let mut buffer: [c_char; 512] = [0; 512];
        for (slot, byte) in buffer.iter_mut().zip("Décode failed\0ignored".bytes()) {
            *slot = c_char::from_ne_bytes([byte]);
        }
        assert_eq!(native_error(&buffer), "Décode failed");
        assert_eq!(
            native_error(&[c_char::from_ne_bytes(*b"x"); 512]),
            "x".repeat(512)
        );
        assert_eq!(native_error(&[0; 512]), "");
        buffer[0] = c_char::from_ne_bytes([0xff]);
        buffer[1] = 0;
        assert_eq!(native_error(&buffer), "\u{fffd}");
    }
    #[test]
    fn linked_backend_contains_audio_only_and_no_protocols() {
        let info = backend_info();
        assert_eq!(info.chromaprint, "1.6.2");
        assert_eq!(info.ffmpeg, "9.0.2");
        assert_eq!(info.license, "LGPL version 2.1 or later");
        assert_eq!(info.audio_decoders, 218);
        assert_eq!(info.non_audio_decoders, 0);
        assert_eq!(info.protocols, 0);
    }
}
