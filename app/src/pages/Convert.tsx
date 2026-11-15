import { useState } from "react";
import type { PageProps } from "../App";
import { api, pickers } from "../api";
import type { ConvertInfo } from "../types";
import { Btn, Chip, Empty, fmtBytes } from "../ui";

export default function Convert({ snap, toast, reload }: PageProps) {
  const [info, setInfo] = useState<ConvertInfo | null>(null);
  const [key, setKey] = useState("");
  const [chunkMb, setChunkMb] = useState(Math.round(snap.config.split_bytes / (1024 * 1024)) || 4095);
  const [busy, setBusy] = useState(false);

  async function inspect(path: string) {
    try {
      setInfo(await api.convertInspect(path));
    } catch (err) {
      toast(`inspect failed: ${String(err)}`);
    }
  }

  async function pickFile() {
    const file = await pickers.pickFile("Choose an ISO, PKG, ZIP or split part", ["iso", "pkg", "zip", "0"]);
    if (file) await inspect(file);
  }

  async function pickFolder() {
    const folder = await pickers.pickDirectory("Choose a game folder");
    if (folder) await inspect(folder);
  }

  async function run(kind: string, out: string) {
    if (!info) return;
    setBusy(true);
    try {
      await api.convertRun(kind, info.path, out, key || undefined, chunkMb);
      toast(`${kind} started`);
      reload();
    } catch (err) {
      toast(`${kind} failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  async function buildIso() {
    if (!info) return;
    const out = await pickers.pickSavePath("Save ISO", `${info.name}.iso`, ["iso"]);
    if (out) await run("folder_to_iso", out);
  }

  async function extractIso() {
    if (!info) return;
    const out = await pickers.pickDirectory("Choose an output folder");
    if (out) await run("iso_to_folder", out);
  }

  async function splitFile() {
    if (!info) return;
    await run("split", info.path);
  }

  async function mergeParts() {
    if (!info) return;
    const out = await pickers.pickSavePath("Save merged file", info.name.replace(/\.\d+$/, ""), ["iso", "pkg"]);
    if (out) await run("merge", out);
  }

  async function decrypt() {
    if (!info) return;
    const out = await pickers.pickSavePath("Save decrypted ISO", info.name.replace(/\.iso$/i, " (decrypted).iso"), ["iso"]);
    if (out) await run("decrypt", out);
  }

  async function install() {
    if (!info) return;
    try {
      await api.installPath(info.path);
      toast(`sending ${info.name} to the console`);
      reload();
    } catch (err) {
      toast(`install failed: ${String(err)}`);
    }
  }

  return (
    <div className="page">
      <div className="panel">
        <div className="panel-head">
          <h2>Conversions</h2>
          <div className="spacer" />
          <Btn small onClick={() => void pickFile()}>Choose file</Btn>
          <Btn small onClick={() => void pickFolder()}>Choose folder</Btn>
        </div>

        {!info ? (
          <Empty
            title="Pick something to convert"
            hint="PS3 game folders to ISO, ISO to folder, FAT32 splits and merges, and Redump decryption."
          />
        ) : (
          <>
            <div className="rows">
              <div className="row">
                <div>
                  <div className="title mono">{info.name}</div>
                  <div className="sub mono" title={info.path}>{info.path}</div>
                </div>
                <div className="grow" />
                <Chip tone="laser">{info.kind}</Chip>
                <Chip tone={info.system === "unknown" ? "warn" : "good"}>{info.system}</Chip>
                {info.encrypted ? <Chip tone="bad">encrypted</Chip> : null}
                <span className="size mono">{fmtBytes(info.size_bytes)}</span>
              </div>
            </div>

            <div className="stats" style={{ marginTop: 12 }}>
              <Stat k="title id" v={info.title_id ?? "-"} />
              <Stat k="parts" v={info.parts ? String(info.parts) : "-"} />
              <Stat k="destination" v={destination(info)} />
            </div>

            <div className="quick" style={{ marginTop: 14 }}>
              {info.kind === "folder" ? (
                <Btn variant="primary" disabled={busy} onClick={() => void buildIso()}>
                  Build ISO
                </Btn>
              ) : null}
              {info.kind === "iso" ? (
                <>
                  <Btn variant="primary" disabled={busy} onClick={() => void extractIso()}>
                    Extract to folder
                  </Btn>
                  <Btn disabled={busy} onClick={() => void splitFile()}>
                    Split for FAT32
                  </Btn>
                  {info.encrypted ? (
                    <Btn disabled={busy} onClick={() => void decrypt()}>
                      Decrypt
                    </Btn>
                  ) : null}
                </>
              ) : null}
              {info.kind === "split" ? (
                <Btn variant="primary" disabled={busy} onClick={() => void mergeParts()}>
                  Merge parts
                </Btn>
              ) : null}
              {info.kind === "pkg" || info.kind === "zip" ? (
                <Btn disabled={busy} onClick={() => void splitFile()}>
                  Split for FAT32
                </Btn>
              ) : null}
              {["pkg", "iso", "folder", "zip"].includes(info.kind) ? (
                <Btn variant="primary" disabled={busy} onClick={() => void install()}>
                  Install on PS3
                </Btn>
              ) : null}
            </div>

            {info.encrypted ? (
              <div className="field" style={{ marginTop: 14 }}>
                <label>Redump disc key (leave empty to look it up in ps3-resources)</label>
                <input
                  type="text"
                  className="mono"
                  placeholder="32 hex characters"
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                />
              </div>
            ) : null}

            <div className="field" style={{ marginTop: 14 }}>
              <label>Split chunk size (MiB)</label>
              <input
                type="number"
                value={chunkMb}
                onChange={(e) => setChunkMb(Number(e.target.value))}
              />
            </div>
          </>
        )}
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>What each conversion does</h2>
        </div>
        <div className="checks">
          <Note name="Folder to ISO" detail="Builds ISO9660 with a Joliet tree, so long and lowercase filenames survive; multi-extent for files over 4 GiB." />
          <Note name="ISO to folder" detail="Extracts the ISO's filesystem to a JB-format game folder." />
          <Note name="Split for FAT32" detail={`Writes <name>.0, <name>.1 ... at ${chunkMb} MiB, the layout PS3 file managers reassemble.`} />
          <Note name="Merge parts" detail="Joins a split set back into one ISO or PKG." />
          <Note name="Decrypt" detail="Redump AES-128-CBC region decryption using the dkey, then clears the map so the image is plaintext." />
        </div>
      </div>
    </div>
  );
}

function destination(info: ConvertInfo): string {
  if (info.kind === "pkg") return "/dev_hdd0/packages";
  if (info.kind === "folder") return "/dev_hdd0/GAMES";
  if (info.system === "ps2") return "/dev_hdd0/PS2ISO";
  if (info.system === "ps1") return "/dev_hdd0/PSXISO";
  if (info.system === "psp") return "/dev_hdd0/PSPISO";
  if (info.kind === "iso") return "/dev_hdd0/PS3ISO";
  return "-";
}

function Stat({ k, v }: { k: string; v: string }) {
  return (
    <div className="stat">
      <div className="v">{v}</div>
      <div className="k">{k}</div>
    </div>
  );
}

function Note({ name, detail }: { name: string; detail: string }) {
  return (
    <div className="check">
      <span className="status-dot idle" />
      <div>
        <div className="name">{name}</div>
        <div className="detail">{detail}</div>
      </div>
    </div>
  );
}
