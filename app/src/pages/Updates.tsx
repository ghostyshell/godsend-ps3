import { useEffect, useState } from "react";
import type { PageProps } from "../App";
import { api } from "../api";
import type { GameEntry, UpdateEntry } from "../types";
import { Btn, Chip, Empty, fmtBytes } from "../ui";

export default function Updates({ toast, reload }: PageProps) {
  const [titleId, setTitleId] = useState("");
  const [rows, setRows] = useState<UpdateEntry[] | null>(null);
  const [games, setGames] = useState<GameEntry[]>([]);

  useEffect(() => {
    void api
      .libraryScan()
      .then((view) => setGames(view.games.filter((g) => g.title_id).slice(0, 30)))
      .catch(() => setGames([]));
  }, []);

  async function load(id = titleId) {
    if (!id.trim()) return;
    setTitleId(id);
    setRows(await api.updatesList(id.trim()));
  }

  return (
    <div className="page">
      <div className="panel">
        <div className="panel-head">
          <h2>Official updates</h2>
          <div className="spacer" />
        </div>
        <div className="quick">
          <input
            type="text"
            className="search mono"
            placeholder="BLES01234"
            value={titleId}
            onChange={(e) => setTitleId(e.target.value.toUpperCase())}
            onKeyDown={(e) => {
              if (e.key === "Enter") void load();
            }}
          />
          <Btn variant="primary" onClick={() => void load()}>
            Look up
          </Btn>
        </div>
        <p style={{ marginBottom: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
          Updates are Sony's own PKGs, indexed by ps3-resources. The newest is
          installed through webMAN.
        </p>
      </div>

      {games.length > 0 ? (
        <div className="panel">
          <div className="panel-head">
            <h2>From the library</h2>
          </div>
          <div className="quick">
            {games.map((game) => (
              <button
                key={game.id}
                className="chip"
                onClick={() => void load(game.title_id ?? "")}
                title={game.title}
              >
                <span className="dot" />
                {game.title_id}
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {rows === null ? null : rows.length === 0 ? (
        <Empty
          title="No updates indexed"
          hint={`Nothing in ps3-resources for ${titleId}. Retail discs sometimes use a different update title id.`}
        />
      ) : (
        <div className="rows">
          {rows.map((row, index) => (
            <div className="row" key={`${row.titleId}-${row.version}-${index}`}>
              <div>
                <div className="title mono">{row.version}</div>
                <div className="sub">{row.name || row.titleId}</div>
              </div>
              <div className="grow" />
              <span className="size mono">{fmtBytes(row.sizeBytes)}</span>
              {index === 0 ? <Chip tone="laser">newest</Chip> : null}
              <Btn
                small
                variant="primary"
                onClick={async () => {
                  try {
                    await api.updateInstall(row.titleId);
                    toast(`installing ${row.version}`);
                    reload();
                  } catch (err) {
                    toast(`install failed: ${String(err)}`);
                  }
                }}
              >
                Install
              </Btn>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
