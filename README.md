<div align="center">
<h1>SteamDepotDownloaderGUI</h1>
<h4>A graphical wrapper for DepotDownloader, designed to make downloading older versions of Steam games easy.</h4>

→ <a href="https://www.youtube.com/watch?v=H2COwT5OUOo" target="_blank"><b>Tutorial</b></a> ~ 
<a href="https://depotdownloader.aphex.cc/" target="_blank"><b>Website</b></a> ~ 
<a href="https://depotdownloader.com/safety/" target="_blank"><b>Safety</b></a> ~ 
<a href="https://depotdownloader.com/how-it-works/" target="_blank"><b>How it works</b></a> ~ 
<a href="https://www.youtube.com/watch?v=ogiDAuH3VdY" target="_blank"><b>Downgrade Subnautica</b></a> ←
</h4>


<a href="https://img.shields.io/github/last-commit/mmvanheusden/SteamDepotDownloaderGUI?color=crimson"><img src="https://img.shields.io/github/last-commit/mmvanheusden/SteamDepotDownloaderGUI?color=crimson" alt="Last contribution badge"></a>
<a href="https://github.com/FDeghy/SteamDepotDownloaderGUI/releases/latest"><img src="https://img.shields.io/badge/Download -ffbd03?style=for-the-badge&logo=data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmHAAAABmJLR0QA/wD/AP+gvaeTAAABd0lEQVRoge2XwUrDQBCGPz3Ee++2UBSvdsGbeLHeFX0lPQhei+8g2AfwSVREK+LJ1oqHUGjRQ7Z0laRJNtldKPPBwoTM7D9/sks2IAiCsKrsAK/AC7DtSmTd1cTAKbAJtIAzVyIuDURGvOFKxKUBL4iB0IiB0IiB0IiB0IiB0IiB0IiB0IgBoA08Ak+AKpCvdO6AGn416zBwDGyRGLljuQmlc9pAEzipKl6HgVtgpOMGSYOdlLyOvtfQ1yPgpgb9WlDAEPjRYwj0jet+yv0iy80ru8AHiyazxiewF6jHXPJMeG3+CoiBy5J1/5dTlWVzAXwD5yXrAJho4YlFbdqesFnzsa6PLWr/PD0bFPAA3GO/YXN7WMspLpLnktweVvooMTPiKDPLHabmLCtpmYF3I96v3E55Doz4zWaCaxYb6Bno4udNRMCR1pzr92wmagFfZH+UfI0xycHPim5gE2Pg0Lb5OU2SVzgAph6anmqtHhWevCAIgh9+AdLMtu/CZhHJAAAAAElFTkSuQmCC" alt="Download latest release badge"></a>
<a href="https://github.com/FDeghy/SteamDepotDownloaderGUI/releases/latest"><img src="https://img.shields.io/github/downloads/FDeghy/SteamDepotDownloaderGUI/total?color=orange&label=downloads" alt="Download count badge"></a>


<img src="https://github.com/user-attachments/assets/e2de938d-e54d-4f30-a405-b0d9d2d8e4d9" alt="Steam downgrader interface" />
</div>

## Features
> ↓ *The downloader in action:*<br>
> <img src="https://github.com/user-attachments/assets/7d8a97b8-79df-4c54-a74b-fb7b4b85913c" alt="Steam downgrader process" style="width: 25svw;"/>


### Cross-platform support
| OS      | Support |
|---------|---------|
| Windows | ✅      |
| Linux   | ✅      |
| macOS   | ✅      |


> [!TIP]
> ## Tutorials
> * https://www.youtube.com/watch?v=H2COwT5OUOo How to download older versions of Steam games tutorial
> * https://www.youtube.com/watch?v=ogiDAuH3VdY How to download older versions of Subnautica tutorial

## Installation
> [!CAUTION]  
> This GitHub repository is the only official place to download this software.  
> If you have paid for this software, or downloaded this from a different place than here, **you should reconsider your digital literacy skills, and are at risk of having your Steam account stolen**.


### Windows
Download the [latest Windows release](https://github.com/FDeghy/SteamDepotDownloaderGUI/releases/latest).


### Linux/Steam Deck
Download the [latest Linux release](https://github.com/FDeghy/SteamDepotDownloaderGUI/releases/latest).

## Download and update from Lua or ZIP

Choose **Import Lua or ZIP…**, review the resolved IDs, and select the installation folder. An empty folder starts a fresh download. Choosing an existing installation enables checksum verification and chunk reuse. The manual ID and SteamDB clipboard workflows retain their output-folder settings.

Lua input accepts literal `addappid`, `addtoken`, and `setManifestId` declarations, with optional trailing semicolons and line comments. A numeric Lua filename supplies the App ID. Otherwise declare one unambiguous app. For example, this input selects Valve's Half-Life Dedicated Server for Windows.

```lua
addappid(90)
setManifestId(5, "2602921513184876182")
```

A ZIP can contain one or more Steam protobuf manifests named `<DepotID>_<ManifestID>.manifest`. Supply the App ID in the ZIP filename (such as `90.zip`) or include a Lua declaration file. The importer checks the embedded depot and manifest IDs, reconciles any Lua declarations, and discards the temporary extracted files after inspection.

The GUI passes the IDs to DepotDownloaderMod in one download. Steam supplies the authorized depot keys and download manifests. Imported keys, access tokens and manifest file paths are never passed to the engine. Use a Steam username for restricted depots. Leaving the password empty lets you enter it in the terminal. Anonymous access uses Steam's dedicated-server subscription.

## Build from source

Install the Tauri platform prerequisites, Git, Node.js, pnpm, Rust, and the .NET 9 SDK. Run these commands from the repository root.

```sh
pnpm install --frozen-lockfile
pnpm tauri build
```

The Cargo build script checks out the exact DepotDownloaderMod revision in [engine-revision](src-tauri/engine-revision) and publishes a self-contained executable for the Rust target. Tauri bundles that sidecar and its license. Git and NuGet access are required when the engine build runs. A fetch, revision check or publish failure stops the build. The GUI displays the bundled engine's version and commit below the terminal.

## Credits
This software makes use of the following projects:
- [DepotDownloaderMod](https://github.com/SteamAutoCracks/DepotDownloaderMod)
- [**DepotDownloader**](https://depotdownloader.com/) :heart:
- [Tauri](https://tauri.app)
- [Preact](https://preactjs.com)
- [Hubut Sans](https://github.com/github/hubot-sans) under [license](https://github.com/github/hubot-sans/blob/05d5ea150c20e6434485db8ffd2277ed18a9e911/LICENSE) 


## Donate
You can donate [here](paypal.me/onderkin) or through the **donate** button in the interface.


## Contribute
Every pull request is welcome! ;)  
Please cleanup the code using:
```console
$ pnpm eslint --fix src/
```
Please also sort Tailwind classes using [RustyWind](https://github.com/avencera/rustywind).


<a href="https://www.gnu.org/licenses/gpl-3.0" target="_blank">
  <img src="https://github.com/mmvanheusden/SteamDepotDownloaderGUI/assets/50550545/b5649b7f-ea49-45c4-b0cd-5f3788dcd6ca" height="40px" alt="GPL v3">
</a>
