import { useCallback, useEffect, useMemo, useState } from "react";
import type { PageProps } from "../App";
import { api } from "../api";
import type { GameEntry, LibraryView, UpdateEntry } from "../types";
import { Btn, Empty, GameIcon, RemoteCover, Seg, confirmDialog, fmtBytes } from "../ui";

type Filter = "all" | "ps3" | "ps2" | "ps1" | "pkg";

export default function Library({ snap, toast, refreshKey, reload, go }: PageProps) {
  const [view, setView] = useState<LibraryView | null>(null);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [updatesFor, setUpdatesFor] = useState<string | null>(null);
  const [updates, setUpdates] = useState<UpdateEntry[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setView(await api.libraryScan());
    } catch (err) {
      toast(`library scan failed: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load, refreshKey]);

  const entries = useMemo(() => {
    if (!view) return [];
    const all = [...view.games, ...view.packages];
    return all.filter((entry) => {
      if (filter !== "all") {
        if (filter === "pkg" && entry.kind !== "pkg") return false;
        if (filter !== "pkg" && entry.system !== filter) return false;
      }
      if (query.trim() && !entry.title.toLowerCase().includes(query.trim().toLowerCase())) return false;
      return true;
    });
  }, [view, filter, query]);

  function coverUrl(entry: GameEntry): string | null {
    // The R2 cover set only indexes PS3 title ids; PS2 and PS1 fall back to a monogram.
    if (entry.title_id && entry.system === "ps3") {
      return `${snap.config.covers_base.replace(/\/$/, "")}/ps3/repo/COV/${entry.title_id}.JPG`;
    }
    return null;
  }

  async function mount(entry: GameEntry, launch: boolean) {
    try {
      await api.mountGame(entry.path, launch);
      toast(launch ? `launching ${entry.title}` : `mounted ${entry.title}`);
    } catch (err) {
      toast(`mount failed: ${String(err)}`);
    }
  }

  async function showUpdates(entry: GameEntry) {
    if (!entry.title_id) {
      toast("this entry has no title id; updates need one");
      return;
    }
    setUpdatesFor(entry.title_id);
    setUpdates(await api.updatesList(entry.title_id));
  }

  async function remove(entry: GameEntry) {
    if (!confirmDialog(`Delete ${entry.title}?`, `Removes ${entry.path} from the console. This cannot be undone.`)) {
      return;
    }
    try {
      await api.libraryDelete(entry.path, entry.kind === "game" || entry.kind === "homebrew");
      toast(`deleted ${entry.title}`);
      void load();
    } catch (err) {
      toast(`delete failed: ${String(err)}`);
    }
  }

  return (
    <div className="page">
      <div className="panel">
        <div className="panel-head">
          <h2>Installed library</h2>
          <div className="spacer" />
          <input
            className="search"
            type="text"
            aria-label="Filter by title"
            placeholder="Filter by title"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Btn small disabled={loading} onClick={() => void load()}>
            {loading ? "Scanning..." : "Rescan"}
          </Btn>
        </div>
        <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
          <Seg<Filter>
            ariaLabel="Library filter"
            value={filter}
            onChange={setFilter}
            options={[
              { value: "all", label: "All" },
              { value: "ps3", label: "PS3" },
              { value: "ps2", label: "PS2" },
              { value: "ps1", label: "PS1" },
              { value: "pkg", label: "Packages" },
            ]}
          />
          <span className="mono" style={{ color: "var(--text-faint)", fontSize: 12 }}>
            {view ? `${view.games.length} games - ${view.packages.length} packages` : "not scanned"}
          </span>
        </div>
      </div>

      {updatesFor ? (
        <div className="panel">
          <div className="panel-head">
            <h2>Updates for {updatesFor}</h2>
            <div className="spacer" />
            <Btn small onClick={() => setUpdatesFor(null)}>
              Close
            </Btn>
          </div>
          {updates.length === 0 ? (
            <Empty title="No updates indexed" hint="ps3-resources has no package for this title id." />
          ) : (
            <div className="rows">
              {updates.map((update) => (
                <div className="row" key={`${update.titleId}-${update.version}`}>
                  <div>
                    <div className="title mono">{update.version}</div>
                    <div className="sub">{update.name || update.titleId}</div>
                  </div>
                  <div className="grow" />
                  <span className="size mono">{fmtBytes(update.sizeBytes)}</span>
                  <Btn
                    small
                    variant="primary"
                    onClick={async () => {
                      await api.updateInstall(update.titleId);
                      toast(`installing update ${update.version}`);
                      reload();
                    }}
                  >
                    Install
                  </Btn>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : null}

      {entries.length === 0 ? (
        <Empty
          title={loading ? "Scanning the console..." : "Nothing here yet"}
          hint="Games in /dev_hdd0/GAMES, PS3ISO, PS2ISO, PSXISO and packages in /dev_hdd0/packages appear here."
          action={<Btn variant="primary" onClick={() => go("downloads")}>Send something to the console</Btn>}
        />
      ) : (
        <div className="grid">
          {entries.map((entry) => {
            const cover = coverUrl(entry);
            return (
              <article className="tile" key={entry.id}>
                <div className="cover">
                  {entry.icon_path ? (
                    <GameIcon path={entry.icon_path} title={entry.title} />
                  ) : (
                    <RemoteCover url={cover} title={entry.title} />
                  )}
                </div>
                <div className="body">
                  <div className="t" title={entry.title}>
                    {entry.title}
                  </div>
                  <div className="s">
                    {entry.system.toUpperCase()} - {entry.kind} -{" "}
                    {entry.size_bytes ? fmtBytes(entry.size_bytes) : entry.device}
                  </div>
                  <div className="actions">
                    {entry.kind === "pkg" ? (
                      <Btn
                        small
                        variant="primary"
                        onClick={async () => {
                          try {
                            await api.installRemote(entry.path);
                            toast(`installing ${entry.title}`);
                          } catch (err) {
                            toast(`install failed: ${String(err)}`);
                          }
                        }}
                      >
                        Install
                      </Btn>
                    ) : (
                      <>
                        <Btn small onClick={() => void mount(entry, false)}>
                          Mount
                        </Btn>
                        <Btn small variant="primary" onClick={() => void mount(entry, true)}>
                          Play
                        </Btn>
                      </>
                    )}
                    {entry.title_id ? (
                      <Btn small onClick={() => void showUpdates(entry)}>
                        Updates
                      </Btn>
                    ) : null}
                    <Btn small variant="danger" onClick={() => void remove(entry)}>
                      Delete
                    </Btn>
                  </div>
                  <div className="s mono" title={entry.path}>
                    {entry.title_id ?? entry.path.split("/").pop()}
                  </div>
                </div>
              </article>
            );
          })}
        </div>
      )}

      {view && view.roots.length > 0 ? (
        <p className="mono" style={{ color: "var(--text-faint)", fontSize: 11.5 }}>
          scanned: {view.roots.join("  ")}
        </p>
      ) : null}
    </div>
  );
}
