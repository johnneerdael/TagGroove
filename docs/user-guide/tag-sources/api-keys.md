# API keys and sign-ins for tag sources

Only fanart.tv takes a key from you, and only where it helps. AcoustID and
Last.fm need nothing: TagGroove has its own keys for both. You can sign in to
your Last.fm account instead of entering a key. Discogs has its own sign-in; see
[Discogs](discogs.md). Other sources need no key.

<!-- figure:source-api-keys -->
<figure markdown>
![The API keys dialog with the Last.fm account sign-in and a section for the fanart.tv key](../assets/screens/source-api-keys-light.png#only-light)
![The API keys dialog with the Last.fm account sign-in and a section for the fanart.tv key](../assets/screens/source-api-keys-dark.png#only-dark)
<figcaption>The Last.fm section signs you in; the fanart.tv section says whether this build includes a key.</figcaption>
</figure>
<!-- /figure -->

Open **Tag Sources → API keys…**. The fanart.tv section says whether this build
includes a key. Without a built-in or saved key, fanart.tv stays unavailable.

Enter your key and choose **Save**. An empty box keeps the saved key;
**Remove the saved key** deletes it. Saved keys are not displayed again. Your
personal fanart.tv key accompanies a built-in project key; without a project key,
your key is used as the project key.

The dialog provides the registration address. Availability depends on the
installed build and provider; do not assume every downloaded or locally
compiled build includes working keys.

## Why AcoustID has no key box

AcoustID has two kinds of keys. An application key lets a program look
fingerprints up, and TagGroove carries its own. A personal user key is only for
submitting new fingerprints to AcoustID. TagGroove never submits, so there is
nothing to enter. See [AcoustID fingerprints](acoustid.md).

## Last.fm account { #lastfm-account }

Last.fm does not ask you for a key. TagGroove uses its own application key, and
the **Last.fm account** section of the dialog only signs you in:

1. Choose **Sign in to Last.fm…**. Your browser opens Last.fm's page that asks
   whether TagGroove may access your account.
2. Approve it on that page, then return to TagGroove and choose **Finish**.
3. The section now says **Signed in as** and your Last.fm user name.

Choose **Sign out** to remove the sign-in from TagGroove. To withdraw TagGroove's
access on Last.fm's side as well, remove it in your Last.fm account's
connected-applications settings. Your Last.fm password is typed only on
Last.fm's own page and is never seen by TagGroove.

Last.fm keeps working without signing in. If the section says this build has no
Last.fm application key, the Last.fm source is unavailable in that build.
