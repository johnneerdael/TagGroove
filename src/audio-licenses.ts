import license0 from './licenses/audio/chromaprint-LICENSE.md?raw';
import license1 from './licenses/audio/chromaprint-src_3rdparty_kissfft_COPYING?raw';
import license2 from './licenses/audio/chromaprint-src_3rdparty_kissfft_LICENSES_BSD-3-Clause?raw';
import license3 from './licenses/audio/ffmpeg-COPYING.LGPLv2.1?raw';
import license4 from './licenses/audio/ffmpeg-LICENSE.md?raw';
import license5 from './licenses/audio/zlib-LICENSE?raw';
import license6 from './licenses/audio/bzip2-LICENSE?raw';
import licenseBoost from '../crates/regex-compat/vendor/boost-regex/LICENSE_1_0.txt?raw';
import licenseDotnet from './licenses/dotnet-runtime-MIT.txt?raw';
import dependencyNotices from '../THIRD_PARTY_DEPENDENCIES.md?raw';
import androidNotices from '../THIRD_PARTY_ANDROID.md?raw';
import nativeNotices from '../THIRD_PARTY_NATIVE.md?raw';
import linuxNotices from '../THIRD_PARTY_LINUX.md?raw';
export const audioLicenses = [
  {title:"Chromaprint 1.6.2 — license and notices",text:license0},
  {title:"KissFFT — copyright notices",text:license1},
  {title:"KissFFT — BSD 3-Clause license",text:license2},
  {title:"FFmpeg 9.0.2 — LGPL 2.1 license",text:license3},
  {title:"FFmpeg 9.0.2 — component license notes",text:license4},
  {title:"zlib 1.3.2 — license",text:license5},
  {title:"bzip2 1.0.8 — license",text:license6},
  {title:"Boost.Regex 1.89.0 — license and notices",text:licenseBoost},
  {title:"Locale data (.NET runtime v10.0.0) — MIT license",text:licenseDotnet},
  {title:"Dependency licenses and notices",text:dependencyNotices},
  {title:"Android/JVM licenses and notices",text:androidNotices},
  {title:"Native runtime and installer notices",text:nativeNotices+'\n\n'+linuxNotices},
];
