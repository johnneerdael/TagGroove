# API keys for tag sources

AcoustID, Last.fm and fanart.tv need a key before they can be used. Discogs
has its own sign-in; see [Discogs](discogs.md). Other sources need no key.

<!-- figure:source-api-keys -->
<figure markdown>
![The API keys dialog with sections for AcoustID, Last.fm and fanart.tv, each showing whether a key is saved](../assets/screens/source-api-keys-light.png#only-light)
![The API keys dialog with sections for AcoustID, Last.fm and fanart.tv, each showing whether a key is saved](../assets/screens/source-api-keys-dark.png#only-dark)
<figcaption>Each section says whether this build includes a key and links to the provider's registration page.</figcaption>
</figure>
<!-- /figure -->

Open **Tag Sources → API keys…** for AcoustID, Last.fm and fanart.tv. Each
section says whether this build includes a key. Without a built-in or saved
key, the corresponding source stays unavailable.

Enter your key and choose **Save**. An empty box keeps the saved key;
**Remove the saved key** deletes it. Saved keys are not displayed again.
Your AcoustID or Last.fm key takes precedence over a built-in key. For
fanart.tv, your personal key accompanies a built-in project key; without a
project key, your key is used as the project key.

The dialog provides registration addresses. Availability depends on the
installed build and provider; do not assume every downloaded or locally
compiled build includes working keys.
