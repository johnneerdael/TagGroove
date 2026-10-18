# Provider-document read preferences

R19 requires Android provider reads to honor the existing tag-kind and field
mapping preferences. Before this change, provider metadata descriptions used
tag-core's default readers. Desktop settings readers already passed the selected
options and mappings explicitly.

Keep transport, durable records, recovery and source-publication rules intact.
Give the store an options supplier, passed as `settings::tag_options` by the
Android adapter. Read the current options on each read rather than caching the
configuration at startup. Use the same captured options for a working-copy read
and its permitted verified-baseline fallback. Apply the shared core's mapped
full/summary readers at every provider read site; retain provider URI projection.

Verify the real persistence engine with filesystem-backed provider fixtures:
conflicting MPEG and APE-family tag kinds, custom field mappings, changing options
after import, duplicate imports, reload, post-publication descriptions, both Save
a copy versions and restart recovery. Cover readable retained edits and the
permitted baseline fallback separately. Exercise the settings-backed command
adapter as well as an isolated supplier. Confirm unchanged provider bytes for
read-only operations and preserve existing failure tests. These tests do not
establish Android device, SAF, cloud-provider, iOS or original-Mp3tag parity.

Preserve PR #53's merged pending-cover and pending-field copy writes, reverse
field mappings and single write-policy snapshot. This change covers the returned
metadata descriptions as well. Keep provider transport, baseline lifetime and
publication authorization unchanged; introduce no new startup state mechanism.
PR #47's branding hold has ended. Follow the current contribution workflow:
require clean Codex code review and protected CI on the exact final head before
merge. Later parity branches remain separate work.
