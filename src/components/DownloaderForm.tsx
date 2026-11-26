import { Icon } from "@iconify-icon/react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useContext, useState } from "preact/hooks";
import type { Imported } from "../App";
import { AppContext } from "../context";
import { FileInput, NumberInput, TextInput } from "./FormInput";

export function DownloaderForm({ imported, setImported, onOutput }: {
    imported?: Imported, setImported: (value?: Imported) => void, onOutput: (data: number[]) => void
}) {
    const context = useContext(AppContext);
    const [downloading, setDownloading] = context.downloading!;
    const [loading, setLoading] = useState(false);
    const [canCancel, setCanCancel] = useState(false);
    const [status, setStatus] = useState("");
    const [source, setSource] = useState("");
    const busy = Boolean(downloading || loading);
    async function chooseInput() {
        setLoading(true);
        try {
            const path = await open({ filters: [{ name: "Steam download input", extensions: ["lua", "zip"] }] });
            if (!path) return;
            setImported(undefined);
            setSource("");
            const input = await invoke<Imported>("import_download", { path });
            setImported(input);
            setSource(path);
            setStatus("");
        } catch (e) { setStatus(String(e)); }
        finally { setLoading(false); }
    }
    async function download(event: SubmitEvent) {
        event.preventDefault();
        if (busy) return;
        if (!context.outputLocation![0]) { setStatus("Choose a destination folder."); return; }
        setCanCancel(false);
        setDownloading(true);
        setStatus("Download running");
        try {
            const output = new Channel<number[]>(bytes => {
                if (bytes.length) onOutput(bytes);
                else setCanCancel(true);
            });
            setStatus(await invoke<string>("start_download", { output, steamDownload: {
                username: context.username![0], password: context.password![0],
                appId: imported?.appId ?? context.appId![0],
                depots: imported?.depots ?? [{ id: context.depotId![0], manifest: context.manifestId![0] }],
                branch: context.branch![0], outputLocation: context.outputLocation![0],
                outputDirectoryName: context.appSettings![0].outputDirectoryMode === "Custom"
                    ? context.appSettings![0].outputDirectoryName : undefined,
                noMobileAuth: context.appSettings![0].noMobileAuth, imported: Boolean(imported),
            } }));
        } catch (e) { setStatus(String(e)); }
        finally { setDownloading(false); }
    }
    return <>
        <form class="mb-1" onSubmit={e => void download(e)}>
            <fieldset disabled={busy} class="flex flex-col gap-1">
                <TextInput id="username" label="Steam username" valueState={context.username} placeholder="Leave empty for anonymous access" />
                <TextInput id="password" label="Password" valueState={context.password} placeholder="Or enter it in the terminal" password />
                <div class="flex gap-3 my-1">
                    <button type="button" class="underline" onClick={() => void chooseInput()}>Import Lua or ZIP…</button>
                    {imported && <button type="button" class="underline" onClick={() => { setImported(undefined); setStatus(""); }}>Use manual IDs</button>}
                </div>
                {imported ? <div class="p-2 bg-gray-800 rounded max-h-28 overflow-auto">
                    <p class="break-all">{source}</p><p>App {imported.appId}</p>
                    {imported.depots.map(d => <p key={d.id}>Depot {d.id} · Manifest {d.manifest}</p>)}
                </div> : <>
                    <NumberInput id="appId" label="App ID" valueState={context.appId} required />
                    <NumberInput id="depotId" label="Depot ID" valueState={context.depotId} required />
                    <NumberInput id="manifestId" label="Manifest ID" valueState={context.manifestId} required />
                </>}
                <TextInput id="branch" label="Branch (optional)" valueState={context.branch} placeholder="Public" />
                <FileInput pathState={context.outputLocation!} label={imported ? "Installation folder" : "Output directory"} />
                <p class="text-sm break-all">{context.outputLocation![0]}</p>
                {imported && <p class="text-sm">Choose an empty folder for a fresh download or the installed game folder to verify and update its files.</p>}
            </fieldset>
            <div class="flex gap-1 my-2">
                {downloading
                    ? <button type="button" disabled={!canCancel} class="grow p-1 rounded bg-red-700 disabled:opacity-60" onClick={() => void invoke("cancel_download").catch(e => setStatus(String(e)))}>Cancel download</button>
                    : <button type="submit" disabled={busy} class="grow p-1 rounded bg-green-700">{imported ? "Download or update" : "Download"}</button>}
                <button type="button" aria-label="Settings" disabled={busy} onClick={() => context.showSettings![1](true)} class="p-1 rounded bg-green-700"><Icon icon="heroicons:cog" width="24" /></button>
            </div>
            <p role="status" class="text-sm break-words">{status}</p>
        </form>
        <div class="flex gap-1 justify-between">
            <InternetButton icon="ic:sharp-discord" title="Discord" href="https://discord.com/invite/3qCt4DT5qe" />
            <InternetButton icon="simple-icons:steamdb" title="SteamDB" href="https://steamdb.info/instantsearch" />
            <InternetButton icon="mdi:youtube" title="Tutorials" href="https://youtube.com/playlist?list=PLRAjc5plLScj967hnsYX-I3Vjw9C1v7Ca" />
            <InternetButton icon="bx:donate-heart" title="Donate" href="https://paypal.me/onderkin" />
        </div>
    </>;
}
function InternetButton(
	{title, icon, href, disabled}: {title: string, icon: string, href?: string, disabled?: boolean}
) {
	const onClick = () => {
		if (href) openUrl(href).catch((e) => console.error(e));
	};
  
	return (
		<button disabled={disabled} onClick={onClick} type="button" class="inline-flex gap-0.5 justify-center items-center py-0.5 px-1 font-medium text-white rounded-md border border-gray-600 transition-transform disabled:pointer-events-none grow bg-blue-900/90 text-md hover:bg-blue-900/65 active:bg-blue-900/40 active:scale-103 disabled:bg-red-500/70">
			<Icon icon={icon} height="20"/>{title}
		</button>
	);
}
