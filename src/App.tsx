import {getVersion} from "@tauri-apps/api/app";
import type {Terminal} from "@xterm/xterm";
import {openUrl} from "@tauri-apps/plugin-opener";
import {useRef, useState} from "preact/hooks";
import {DownloaderForm} from "./components/DownloaderForm.tsx";
import {DownloaderOutput} from "./components/DownloaderOutput.tsx";
import {Settings} from "./components/Settings.tsx";
import {AppContext, AppSettings} from "./context.ts";
import "./css/App.css";
import {ClipBoardParserButton} from "./components/ClipboardParser.tsx";

const appVersion = await getVersion();


// Settings defaults are defined here.
const DEFAULT_APP_SETTINGS: AppSettings = {
	outputDirectoryMode: "Manifest ID",
	outputDirectoryName: "",
	noMobileAuth: false,
};


export interface Imported {
    appId: string;
    depots: { id: string; manifest: string }[];
}

function App() {
	const username = useState<string>();
	const password = useState<string>();
	const appId = useState<string>();
	const depotId = useState<string>();
	const manifestId = useState<string>();
	const branch = useState<string>();
	const outputLocation = useState<string>();
	const downloading = useState<boolean>();
    const terminal = useRef<Terminal | null>(null);
    const [imported, setImported] = useState<Imported>();

	const showSettings = useState<boolean>(false);
	const appSettings = useState<AppSettings>(DEFAULT_APP_SETTINGS);
	
	
	return (
		<AppContext.Provider
			value={{
				username,
				password,
				appId,
				depotId,
				manifestId,
				branch,
				outputLocation,
				downloading,
				showSettings,
				appSettings,
			}}
		>
			<main class="absolute top-0 right-0 bottom-0 left-0 p-px text-white select-none bg-[#0d1117]">
				<div hidden={!showSettings[0]}><Settings /></div>
				<div hidden={showSettings[0]}>
					<div class="flex justify-center w-full">
						<div class="inline-flex absolute left-0 pt-1 pl-3">
							<ClipBoardParserButton disabled={Boolean(downloading[0] || imported)}/>
						</div>
						<div class="mb-1 text-4xl font-bold text-center font-['Hubot_Sans']">
							Steam Depot Downloader
						</div>
					</div>
					<div class="flex gap-5 justify-between max-h-screen">
						<div class="pl-3 w-full max-w-1/2">
							<DownloaderForm imported={imported} setImported={setImported} onOutput={data => terminal.current?.write(new Uint8Array(data))} />
						</div>
						<div class="pr-3 w-full max-w-1/2">
							<DownloaderOutput terminal={terminal} downloading={Boolean(downloading[0])} />
						</div>
					</div>
				</div>
				<button tabIndex={-1} type="button" onClick={() => { openUrl(`https://github.com/FDeghy/SteamDepotDownloaderGUI/releases/tag/v${appVersion}`).catch((e) => console.error(e)); }} class="absolute right-0 bottom-0 text-white hover:underline">{`v${appVersion}`}</button>
			</main>
		</AppContext.Provider>
	);
}

export default App;
