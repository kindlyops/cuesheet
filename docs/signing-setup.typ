#set document(title: "Cuesheet — Code Signing Setup", author: "Cuesheet")
#set page(
  paper: "us-letter",
  margin: (x: 2cm, top: 2cm, bottom: 1.8cm),
  footer: context [
    #set text(size: 8pt, fill: luma(50%))
    #line(length: 100%, stroke: 0.5pt + luma(80%))
    #v(3pt)
    #grid(columns: (1fr, auto),
      [Cuesheet code-signing setup],
      [#counter(page).display() / #counter(page).final().first()],
    )
  ],
)
#set text(font: "Libertinus Serif", size: 10.5pt, fill: rgb("#33271C"))
#set par(justify: true, leading: 0.62em)
#show raw: set text(font: "DejaVu Sans Mono", size: 8.5pt, fill: rgb("#7A4419"))
#show link: set text(fill: rgb("#235a68"))

#let fox = rgb("#C95B0C")
#let russet = rgb("#7A4419")
#let cream = rgb("#F7EED7")

// Section heading helper: numbered, fox-accented.
#let step(n, title) = {
  v(6pt)
  block(width: 100%, fill: cream, inset: (x: 10pt, y: 7pt), radius: 3pt)[
    #text(size: 13pt, weight: "bold", fill: russet)[
      #box(fill: fox, inset: (x: 6pt, y: 2pt), radius: 2pt)[#text(fill: white)[#n]]
      #h(4pt) #title
    ]
  ]
  v(3pt)
}

// --- Title band ---
#block(width: 100%, fill: fox, inset: 16pt, radius: 4pt)[
  #text(size: 22pt, weight: "bold", fill: white)[Cuesheet]
  #h(6pt)
  #text(size: 22pt, fill: cream)[Code Signing Setup]
  #v(2pt)
  #text(size: 10pt, fill: cream)[
    How to acquire and configure the secrets that make installers trusted and
    enable auto-updates.
  ]
]

#v(8pt)

These steps are *optional* and can be added at any time. Until you add them,
the macOS and Windows installers still work but are *unsigned* — macOS
Gatekeeper and Windows SmartScreen show a warning on first launch. Each step
below activates automatically the next time the *Release* workflow runs, as
soon as its secrets are present.

#block(fill: luma(96%), inset: 10pt, radius: 3pt, width: 100%)[
  *Where every secret goes.* On GitHub, open your repository and navigate to
  *Settings ▸ Secrets and variables ▸ Actions ▸ New repository secret*. Add
  each one by the exact name shown in bold below. (Direct link:
  #link("https://github.com/kindlyops/cuesheet/settings/secrets/actions")[github.com/kindlyops/cuesheet/settings/secrets/actions].)
]

#step("1", "Updater signing key — do this first")

This keypair lets the app verify and install its own updates. It is separate
from OS code-signing and is the only piece needed for auto-update to work.

+ Generate a keypair (pick a password, or press enter for none):
  #v(2pt)
  ```sh
  npx @tauri-apps/cli signer generate -w ~/.tauri/cuesheet.key
  ```
+ Add two secrets:
  - *`TAURI_SIGNING_PRIVATE_KEY`* — the full contents of the file
    `~/.tauri/cuesheet.key`.
  - *`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`* — the password you chose (leave the
    secret empty if you picked none).
+ Copy the *public* key the command printed, paste it into
  `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`, and commit that
  change.

#step("2", "Apple — macOS signing & notarization")

Removes the Gatekeeper warning on macOS. Requires a paid *Apple Developer
Program* membership (\$99/year).

+ In *Xcode ▸ Settings ▸ Accounts*, or at
  #link("https://developer.apple.com/account/resources/certificates")[developer.apple.com],
  create a *Developer ID Application* certificate.
+ In *Keychain Access*, find that certificate, right-click ▸ *Export*, and save
  it as a `.p12` file with a password.
+ Turn the `.p12` into base64 text for GitHub:
  #v(2pt)
  ```sh
  base64 -i cuesheet.p12 | pbcopy
  ```
+ Create an *app-specific password* at
  #link("https://appleid.apple.com")[appleid.apple.com] ▸ *Sign-In and Security
  ▸ App-Specific Passwords*. Find your *Team ID* on the
  #link("https://developer.apple.com/account")[Membership] page.
+ Add these secrets:
  - *`APPLE_CERTIFICATE`* — the base64 text from step 3.
  - *`APPLE_CERTIFICATE_PASSWORD`* — the `.p12` password from step 2.
  - *`APPLE_SIGNING_IDENTITY`* — e.g. `Developer ID Application: Your Name (TEAMID)`.
  - *`APPLE_ID`* — your Apple account email.
  - *`APPLE_PASSWORD`* — the app-specific password from step 4.
  - *`APPLE_TEAM_ID`* — your 10-character Team ID.

#step("3", "Windows — Authenticode")

Removes the SmartScreen warning on Windows. Requires a code-signing
certificate from a certificate authority (e.g. DigiCert, Sectigo, SSL.com).

+ Buy an *OV code-signing certificate*. Prefer one delivered as a file you can
  export. _Note:_ EV certificates usually ship on a hardware token that cannot
  be exported, so they don't work with this file-based CI flow without a
  cloud-signing service.
+ Export the certificate as a `.pfx` file with a password, then base64-encode it:
  #v(2pt)
  ```powershell
  [Convert]::ToBase64String([IO.File]::ReadAllBytes("cuesheet.pfx")) | Set-Clipboard
  ```
+ Add these secrets:
  - *`WINDOWS_CERTIFICATE`* — the base64 text of the `.pfx`.
  - *`WINDOWS_CERTIFICATE_PASSWORD`* — the `.pfx` password.

#v(8pt)
#line(length: 100%, stroke: 0.5pt + luma(80%))
#v(4pt)

== Quick reference — all secrets

#table(
  columns: (auto, 1fr),
  stroke: none,
  inset: (x: 6pt, y: 4pt),
  fill: (_, row) => if calc.odd(row) { luma(97%) } else { white },
  table.header(
    text(weight: "bold", fill: russet)[Secret name],
    text(weight: "bold", fill: russet)[What it holds],
  ),
  raw("TAURI_SIGNING_PRIVATE_KEY"), [Updater private key file contents],
  raw("TAURI_SIGNING_PRIVATE_KEY_PASSWORD"), [Password for that key (may be empty)],
  raw("APPLE_CERTIFICATE"), [Base64 of the Developer ID `.p12`],
  raw("APPLE_CERTIFICATE_PASSWORD"), [Password for the `.p12`],
  raw("APPLE_SIGNING_IDENTITY"), [`Developer ID Application: Name (TEAMID)`],
  raw("APPLE_ID"), [Apple account email],
  raw("APPLE_PASSWORD"), [App-specific password],
  raw("APPLE_TEAM_ID"), [10-character Apple Team ID],
  raw("WINDOWS_CERTIFICATE"), [Base64 of the `.pfx`],
  raw("WINDOWS_CERTIFICATE_PASSWORD"), [Password for the `.pfx`],
)

#v(6pt)
#block(fill: cream, inset: 10pt, radius: 3pt, width: 100%)[
  *After adding secrets:* re-run the *Release* workflow (Actions ▸ Release ▸
  Run workflow). Apple notarization and Windows signing turn on by themselves
  when their secrets exist; nothing else needs to change. You can add the three
  groups independently and in any order — though the updater key (step 1) is
  the one to do first.
]
