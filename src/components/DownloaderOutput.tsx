import { invoke } from "@tauri-apps/api/core";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef, useState } from "preact/hooks";

export function DownloaderOutput({ terminal, downloading }: {
    terminal: { current: Terminal | null }, downloading: boolean
}) {
    const element = useRef<HTMLDivElement>(null);
    const [engine, setEngine] = useState("");
    useEffect(() => {
        void invoke<string>("engine_version").then(setEngine, e => setEngine(String(e)));
        const term = new Terminal({ fontSize: 10, cursorBlink: true, theme: { background: "rgb(30,33,46)" } });
        terminal.current = term;
        const addon = new FitAddon();
        term.loadAddon(addon);
        term.open(element.current!);
        term.onData(data => { void invoke("async_write_to_pty", { data }).catch(console.error); });
        const resize = () => {
            if (!element.current?.clientWidth) return;
            addon.fit();
            void invoke("async_resize_pty", { rows: term.rows, cols: term.cols }).catch(console.error);
        };
        const observer = new ResizeObserver(resize);
        observer.observe(element.current!);
        return () => { observer.disconnect(); terminal.current = null; term.dispose(); };
    }, [terminal]);

    return <div class="mx-auto mt-2">
        <div class="bg-gray-900 rounded-md border border-gray-300">
            <div class="flex items-center justify-between px-2 py-1">
                <span title={engine}>Download output</span>
                <button type="button" disabled={downloading} onClick={() => terminal.current?.reset()}>Clear</button>
            </div>
            <div ref={element} class="h-[66vh]"></div>
        </div>
        <p class="text-xs break-all text-gray-400 mt-1">{engine}</p>
    </div>;
}
