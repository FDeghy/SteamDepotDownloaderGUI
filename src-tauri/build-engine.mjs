import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const revision = readFileSync(join(root, "engine-revision"), "utf8").trim();
const target = process.argv[2];
const [arch, vendor, platform] = (target ?? "").split("-");
const os = vendor === "apple" && platform === "darwin" ? "osx" : platform;
const rid = { windows: "win", linux: "linux", osx: "osx" }[os];
const cpu = { x86_64: "x64", aarch64: "arm64", i686: "x86", armv7: "arm" }[arch];
if (!rid || !cpu || !/^[0-9a-f]{40}$/.test(revision)) throw new Error("Invalid engine target or revision");

const cwd = mkdtempSync(join(tmpdir(), "depotdownloader-build-"));
const run = (program, args) => execFileSync(program, args, { cwd, stdio: "inherit" });
try {
    run("git", ["init", "--quiet"]);
    run("git", ["fetch", "--quiet", "--depth=1", "https://github.com/SteamAutoCracks/DepotDownloaderMod", revision]);
    run("git", ["checkout", "--quiet", "--detach", "FETCH_HEAD"]);
    if (execFileSync("git", ["rev-parse", "HEAD"], { cwd, encoding: "utf8" }).trim() !== revision)
        throw new Error("Unexpected upstream revision");

    run("dotnet", ["publish", "DepotDownloader/DepotDownloaderMod.csproj", "-c", "Release",
        "-p:PathMap=" + cwd + "=/_/DepotDownloaderMod", "-p:DebugType=None",
        "-r", rid + "-" + cpu, "--self-contained", "true", "-p:PublishSingleFile=true",
        "-p:IncludeNativeLibrariesForSelfExtract=true", "--source", "https://api.nuget.org/v3/index.json", "-o", "publish"]);
    const output = join(root, "binaries");
    const suffix = rid === "win" ? ".exe" : "";
    mkdirSync(output, { recursive: true });
    copyFileSync(join(cwd, "publish", "LICENSE"), join(output, "DepotDownloaderMod-LICENSE"));
    copyFileSync(join(cwd, "publish", "DepotDownloaderMod" + suffix), join(output, "DepotDownloaderMod-" + target + suffix));
} finally {
    rmSync(cwd, { recursive: true, force: true, maxRetries: 3 });
}
