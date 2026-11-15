import { useMemo, useState } from "react";
import type { PageProps } from "../App";
import { api } from "../api";
import { Btn, Chip, Empty, fmtBytes } from "../ui";

export default function Home({ snap, toast, reload, go }: PageProps) {
  const [busy, setBusy] = useState(false);
  const active = snap.statuses.find((s) => s.host_id === snap.active) ?? null;
  const host = snap.hosts.find((h) => h.id === snap.active) ?? null;

  const checks = useMemo(() => snap.doctor ?? [], [snap.doctor]);

  async function action(name: string, arg?: string) {
    setBusy(true);
    try {
      await api.systemAction(name, arg);
      toast(
        name === "notify"
          ? "notification sent"
          : name === "screenshot"
          ? "screenshot requested; it appears in /dev_hdd0/photo"
          : name
      );
    } catch (err) {
      toast(`${name} failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <section className="hero">
        <div className="top">
          <div>
            <div className="eyebrow">CONSOLE</div>
            <h1>{host?.name ?? "No console selected"}</h1>
            <div className="sub">
              {active
                ? `${active.addr} - FTP ${active.ftp_ok ? `${active.ftp_ms} ms` : "offline"}${
                    active.webman_ok ? " - webMAN ready" : " - webMAN missing"
                  }`
                : "Add a PS3 on the Devices page to begin."}
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
            {active ? (
              <>
                <Chip tone={active.ftp_ok ? "good" : "bad"} live={active.ftp_ok}>
                  {active.ftp_ok ? "FTP online" : "FTP offline"}
                </Chip>
                <Chip tone={active.webman_ok ? "good" : "warn"}>
                  {active.webman_ok ? "webMAN" : "no webMAN"}
                </Chip>
              </>
            ) : null}
          </div>
        </div>

        <div className="stats">
          <Stat k="firmware" v={active?.firmware ?? "-"} />
          <Stat k="model" v={active?.model ?? "-"} />
          <Stat
            k="internal hdd"
            v={
              active?.free_bytes !== null && active?.free_bytes !== undefined
                ? `${fmtBytes(active.free_bytes)} free`
                : "unknown"
            }
          />
          <Stat k="webman" v={active?.webman_version ?? "-"} />
        </div>

        <div className="quick">
          <Btn variant="primary" disabled={!active?.webman_ok || busy} onClick={() => go("library")}>
            Open library
          </Btn>
          <Btn disabled={!active?.webman_ok || busy} onClick={() => go("controller")}>
            Virtual pad
          </Btn>
          <Btn disabled={!active?.webman_ok || busy} onClick={() => action("screenshot")}>
            Screenshot
          </Btn>
          <Btn
            disabled={!active?.webman_ok || busy}
            onClick={() => {
              const text = window.prompt("Notification text", "GODsend PS3");
              if (text) void action("notify", text);
            }}
          >
            Notify
          </Btn>
          <Btn disabled={!active || busy} onClick={() => reload()}>
            Rescan
          </Btn>
        </div>
      </section>

      <div className="panel">
        <div className="panel-head">
          <h2>Preflight</h2>
          <div className="spacer" />
          <Btn small disabled={busy} onClick={() => reload()}>
            Re-check
          </Btn>
        </div>
        <div className="checks">
          {checks.map((check) => (
            <div className="check" key={check.name}>
              <span className={`status-dot ${check.ok ? "ok" : "idle"}`} />
              <div>
                <div className="name">{check.name}</div>
                <div className="detail">{check.detail}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>Consoles</h2>
          <div className="spacer" />
          <Btn small onClick={() => go("devices")}>
            Manage
          </Btn>
        </div>
        {snap.statuses.length === 0 ? (
          <Empty
            title="No consoles yet"
            hint="Add the PS3's LAN address. FTP is built into webMAN MOD, multiMAN and IRISMAN."
            action={<Btn variant="primary" onClick={() => go("devices")}>Add a console</Btn>}
          />
        ) : (
          <div className="rows">
            {snap.statuses.map((status) => (
              <div className="row" key={status.host_id}>
                <span className={`status-dot ${status.ftp_ok ? "ok" : "off"}`} />
                <div>
                  <div className="title">{status.name}</div>
                  <div className="sub">
                    {status.addr} - {status.firmware ?? "firmware unknown"}
                  </div>
                </div>
                <div className="grow" />
                <span className="size mono">{status.ftp_ok ? `${status.ftp_ms} ms` : "offline"}</span>
                {status.host_id !== snap.active ? (
                  <Btn
                    small
                    onClick={async () => {
                      await api.hostSetActive(status.host_id);
                      toast(`active console: ${status.name}`);
                      reload();
                    }}
                  >
                    Use
                  </Btn>
                ) : (
                  <Chip tone="laser">active</Chip>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>System</h2>
          <div className="spacer" />
        </div>
        <div className="quick">
          <Btn disabled={!active?.webman_ok || busy} onClick={() => action("refresh")}>
            Refresh XMB
          </Btn>
          <Btn disabled={!active?.webman_ok || busy} onClick={() => action("rebuild")}>
            Rebuild DB
          </Btn>
          <Btn disabled={!active?.webman_ok || busy} onClick={() => action("external-gamedata")}>
            Toggle external gamedata
          </Btn>
          <Btn disabled={!active?.webman_ok || busy} onClick={() => action("reboot-quick")}>
            Quick restart
          </Btn>
          <Btn
            variant="danger"
            disabled={!active?.webman_ok || busy}
            onClick={() => {
              if (window.confirm("Shut down the console?")) void action("shutdown");
            }}
          >
            Power off
          </Btn>
        </div>
      </div>
    </div>
  );
}

function Stat({ k, v }: { k: string; v: string }) {
  return (
    <div className="stat">
      <div className="v">{v}</div>
      <div className="k">{k}</div>
    </div>
  );
}
