//! Ordered, side-effect-free track and disc numbering.
use crate::Fields;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct NumberingItem {
    pub path: String,
    pub fields: Fields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberingOptions {
    pub track_start: u32,
    pub track_digits: u8,
    pub track_total: Option<u32>,
    /// Save as the total the last track number before the track counter
    /// restarts, instead of `track_total`: per folder for Ctrl+Shift+K.
    #[serde(default)]
    pub count_track_total: bool,
    pub disc_enabled: bool,
    pub disc_start: u32,
    pub disc_digits: u8,
    pub disc_total: Option<u32>,
    pub next_disc_on_directory: bool,
    pub next_disc_on_field: Option<String>,
    pub reset_track_on_next_disc: bool,
    pub reset_disc_on_field: Option<String>,
}

impl Default for NumberingOptions {
    fn default() -> Self {
        Self {
            track_start: 1,
            track_digits: 2,
            track_total: None,
            count_track_total: false,
            disc_enabled: false,
            disc_start: 1,
            disc_digits: 1,
            disc_total: None,
            next_disc_on_directory: false,
            next_disc_on_field: None,
            reset_track_on_next_disc: false,
            reset_disc_on_field: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NumberingUpdate {
    pub path: String,
    pub fields: Fields,
}

pub fn plan_numbering(
    items: &[NumberingItem],
    options: &NumberingOptions,
) -> Result<Vec<NumberingUpdate>, String> {
    validate_counter(
        "Track",
        options.track_start,
        options.track_digits,
        options.track_total,
    )?;
    if options.disc_enabled {
        validate_counter(
            "Disc",
            options.disc_start,
            options.disc_digits,
            options.disc_total,
        )?;
    }
    for field in [&options.next_disc_on_field, &options.reset_disc_on_field]
        .into_iter()
        .flatten()
    {
        if field.trim().is_empty() || field.contains('\0') {
            return Err(
                "Counter changes require a tag field name, such as ALBUM or DISCNUMBER".into(),
            );
        }
    }
    let mut track = options.track_start;
    let mut disc = if options.disc_enabled {
        options.disc_start
    } else {
        1
    };
    // Track and disc number of each item and the counter run it belongs to,
    // with the last track number of each run.
    let mut numbers = Vec::with_capacity(items.len());
    let mut runs = vec![options.track_start];
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            let previous = &items[index - 1];
            let reset = field_changed(previous, item, options.reset_disc_on_field.as_deref());
            let next = (options.next_disc_on_directory
                && std::path::Path::new(&previous.path).parent()
                    != std::path::Path::new(&item.path).parent())
                || field_changed(previous, item, options.next_disc_on_field.as_deref());
            if reset {
                disc = if options.disc_enabled {
                    options.disc_start
                } else {
                    1
                };
            } else if next {
                disc = disc
                    .checked_add(1)
                    .ok_or("Disc number exceeds the supported range")?;
            }
            track = if reset || (next && options.reset_track_on_next_disc) {
                runs.push(options.track_start);
                options.track_start
            } else {
                track
                    .checked_add(1)
                    .ok_or("Track number exceeds the supported range")?
            };
        }
        *runs.last_mut().unwrap() = track;
        numbers.push((track, disc, runs.len() - 1));
    }
    let mut updates = Vec::with_capacity(items.len());
    for (item, (track, disc, run)) in items.iter().zip(numbers) {
        let total = if options.count_track_total {
            Some(runs[run])
        } else {
            options.track_total
        };
        let mut fields = Fields::from([(
            "TRACK".into(),
            vec![formatted(track, options.track_digits, total)],
        )]);
        if options.disc_enabled {
            fields.insert(
                "DISCNUMBER".into(),
                vec![formatted(disc, options.disc_digits, options.disc_total)],
            );
        }
        updates.push(NumberingUpdate {
            path: item.path.clone(),
            fields,
        });
    }
    Ok(updates)
}

fn validate_counter(label: &str, start: u32, digits: u8, total: Option<u32>) -> Result<(), String> {
    if start == 0 || !(1..=10).contains(&digits) || total == Some(0) {
        return Err(format!(
            "{label} numbers require a positive start and total, and 1–10 digits"
        ));
    }
    Ok(())
}

fn field_changed(previous: &NumberingItem, current: &NumberingItem, field: Option<&str>) -> bool {
    fn values<'a>(item: &'a NumberingItem, name: &str) -> &'a [String] {
        item.fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, values)| values.as_slice())
            .unwrap_or(&[])
    }
    field.is_some_and(|name| values(previous, name) != values(current, name))
}

fn formatted(value: u32, digits: u8, total: Option<u32>) -> String {
    let mut output = format!("{value:0width$}", width = usize::from(digits));
    if let Some(total) = total {
        output.push_str(&format!("/{total}"));
    }
    output
}
