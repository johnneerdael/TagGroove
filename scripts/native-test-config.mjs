// Build the isolated native test product from the checked-out application name.
import { readFileSync } from 'node:fs';
const {productName} = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const config = {identifier: 'org.opentag.editor.usertest', productName: `${productName} UserTest`};
if (process.env.RUNNER_OS !== 'Linux') {
  config.app = {windows: [{title: productName, width: 1240, height: 800, minWidth: 800, minHeight: 560, backgroundThrottling: 'disabled'}]};
}
console.log(JSON.stringify(config));
