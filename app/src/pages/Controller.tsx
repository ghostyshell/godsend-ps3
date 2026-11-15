import { useCallback, useEffect, useRef, useState } from "react";
import type { PageProps } from "../App";
import { api } from "../api";
import { Btn, Chip, Empty } from "../ui";

const KEY_MAP: Record<string, string> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Enter: "cross",
  Escape: "circle",
  Backspace: "square",
  KeyT: "triangle",
  KeyQ: "l1",
  KeyE: "r1",
  Digit1: "l2",
  Digit3: "r2",
  ShiftLeft: "l3",
  ShiftRight: "r3",
  Tab: "select",
  Space: "start",
  KeyF: "ps",
};

const PRESETS: { label: string; buttons: string[] }[] = [
  { label: "L3 + R3", buttons: ["l3", "r3"] },
  { label: "SELECT + START", buttons: ["select", "start"] },
  { label: "SELECT + CIRCLE", buttons: ["select", "circle"] },
  { label: "R2 + L3 + CIRCLE", buttons: ["r2", "l3", "circle"] },
];

export default function Controller({ snap, toast }: PageProps) {
  const active = snap.statuses.find((s) => s.host_id === snap.active) ?? null;
  const [held, setHeld] = useState<string | null>(null);
  const [combo, setCombo] = useState<string[]>([]);
  const [macro, setMacro] = useState("cross, down, down, cross");
  const heldRef = useRef<string | null>(null);

  const ready = Boolean(active?.webman_ok);

  const release = useCallback(async () => {
    if (heldRef.current) {
      heldRef.current = null;
      setHeld(null);
      try {
        await api.padRelease();
      } catch {
        /* the console may have gone away */
      }
    }
  }, []);

  const hold = useCallback(
    async (button: string) => {
      if (!ready) {
        toast("webMAN is not answering; remote control needs it");
        return;
      }
      heldRef.current = button;
      setHeld(button);
      try {
        await api.padHold(button);
      } catch (err) {
        toast(`pad failed: ${String(err)}`);
        heldRef.current = null;
        setHeld(null);
      }
    },
    [ready, toast]
  );

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      const button = KEY_MAP[e.code];
      if (!button || e.repeat || e.metaKey || e.ctrlKey || e.altKey) return;
      e.preventDefault();
      void hold(button);
    };
    const up = (e: KeyboardEvent) => {
      if (KEY_MAP[e.code]) void release();
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      void release();
    };
  }, [hold, release]);

  async function fireCombo(buttons: string[]) {
    if (!ready) {
      toast("webMAN is not answering");
      return;
    }
    try {
      await api.padCombo(buttons, 140);
      toast(`combo ${buttons.join("+")}`);
    } catch (err) {
      toast(`combo failed: ${String(err)}`);
    }
  }

  return (
    <div className="page">
      <div className="panel">
        <div className="panel-head">
          <h2>Virtual controller</h2>
          <div className="spacer" />
          {active?.webman_ok ? (
            <Chip tone="good" live>
              connected to {active.name}
            </Chip>
          ) : (
            <Chip tone="bad">webMAN required</Chip>
          )}
        </div>

        {!ready ? (
          <Empty
            title="Remote control is offline"
            hint="Install webMAN MOD on the console (Homebrew page) so HTTP commands on port 80 are available."
          />
        ) : null}

        <div className="pad-layout">
          <div className="pad">
            <div className="pad-shoulders">
              <PadButton id="l1" label="L1" held={held} hold={hold} release={release} />
              <PadButton id="l2" label="L2" held={held} hold={hold} release={release} />
              <PadButton id="l3" label="L3" held={held} hold={hold} release={release} />
              <PadButton id="r1" label="R1" held={held} hold={hold} release={release} />
              <PadButton id="r2" label="R2" held={held} hold={hold} release={release} />
              <PadButton id="r3" label="R3" held={held} hold={hold} release={release} />
            </div>

            <div className="pad-main">
              <div className="dpad">
                <PadButton id="up" label="Up" held={held} hold={hold} release={release} className="du" />
                <PadButton id="left" label="Left" held={held} hold={hold} release={release} className="dl" />
                <PadButton id="right" label="Right" held={held} hold={hold} release={release} className="dr" />
                <PadButton id="down" label="Down" held={held} hold={hold} release={release} className="dd" />
              </div>
              <div className="face">
                <PadButton id="triangle" label="Triangle" held={held} hold={hold} release={release} className="ft" />
                <PadButton id="square" label="Square" held={held} hold={hold} release={release} className="fs" />
                <PadButton id="circle" label="Circle" held={held} hold={hold} release={release} className="fc" />
                <PadButton id="cross" label="Cross" held={held} hold={hold} release={release} className="fx" />
              </div>
            </div>

            <div className="pad-center">
              <PadButton id="select" label="Select" held={held} hold={hold} release={release} />
              <PadButton id="ps" label="PS" held={held} hold={hold} release={release} />
              <PadButton id="start" label="Start" held={held} hold={hold} release={release} />
            </div>

            <div className="pad-sticks">
              <div>
                <div className="stick-label">Left stick</div>
                <Analog side="L" ready={ready} />
              </div>
              <div>
                <div className="stick-label">Right stick</div>
                <Analog side="R" ready={ready} />
              </div>
            </div>
          </div>

          <div className="pad-side">
            <div className="panel" style={{ margin: 0 }}>
              <div className="panel-head">
                <h3>Combos</h3>
              </div>
              <div className="quick">
                {PRESETS.map((preset) => (
                  <Btn key={preset.label} small onClick={() => void fireCombo(preset.buttons)}>
                    {preset.label}
                  </Btn>
                ))}
              </div>
              <div className="panel-head" style={{ marginTop: 16 }}>
                <h3>Build a combo</h3>
              </div>
              <div className="quick">
                {["cross", "circle", "square", "triangle", "l1", "r1", "l2", "r2", "l3", "r3", "select", "start"].map(
                  (button) => (
                    <button
                      key={button}
                      className={`chip ${combo.includes(button) ? "laser" : ""}`}
                      onClick={() =>
                        setCombo((cur) =>
                          cur.includes(button) ? cur.filter((b) => b !== button) : [...cur, button]
                        )
                      }
                    >
                      {button}
                    </button>
                  )
                )}
              </div>
              <div className="quick" style={{ marginTop: 10 }}>
                <Btn
                  variant="primary"
                  disabled={combo.length === 0}
                  onClick={() => void fireCombo(combo)}
                >
                  Press {combo.join("+") || "nothing"}
                </Btn>
                <Btn small onClick={() => setCombo([])}>
                  Clear
                </Btn>
              </div>

              <div className="panel-head" style={{ marginTop: 16 }}>
                <h3>Macro</h3>
              </div>
              <p style={{ marginTop: 0, fontSize: 12.5, color: "var(--text-faint)" }}>
                Comma-separated presses. Use <span className="mono">a+b</span> for a chord.
              </p>
              <input
                type="text"
                className="search"
                style={{ width: "100%" }}
                value={macro}
                onChange={(e) => setMacro(e.target.value)}
              />
              <div className="quick" style={{ marginTop: 10 }}>
                <Btn
                  onClick={async () => {
                    try {
                      await api.padMacro(macro);
                      toast("macro sent");
                    } catch (err) {
                      toast(`macro failed: ${String(err)}`);
                    }
                  }}
                >
                  Run macro
                </Btn>
                <Btn
                  onClick={() =>
                    setMacro("up, up, cross, down, cross")
                  }
                >
                  XMB sample
                </Btn>
              </div>

              <div className="panel-head" style={{ marginTop: 16 }}>
                <h3>Console</h3>
              </div>
              <div className="quick">
                <Btn small onClick={() => void fireCombo(["ps"])}>PS button</Btn>
                <Btn
                  small
                  onClick={async () => {
                    try {
                      await api.systemAction("screenshot");
                      toast("screenshot requested");
                    } catch (err) {
                      toast(String(err));
                    }
                  }}
                >
                  Screenshot
                </Btn>
                <Btn
                  small
                  variant="danger"
                  onClick={async () => {
                    if (!window.confirm("Restart the console?")) return;
                    try {
                      await api.systemAction("reboot-quick");
                    } catch (err) {
                      toast(String(err));
                    }
                  }}
                >
                  Restart
                </Btn>
              </div>
              <p style={{ fontSize: 12, color: "var(--text-faint)" }}>
                Keys: arrows d-pad, Enter cross, Esc circle, Backspace square, T
                triangle, Q/R1, 1/3 triggers, Shift sticks, Tab select, Space
                start, F PS.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function PadButton({
  id,
  label,
  held,
  hold,
  release,
  className = "",
}: {
  id: string;
  label: string;
  held: string | null;
  hold: (button: string) => Promise<void>;
  release: () => Promise<void>;
  className?: string;
}) {
  return (
    <button
      className={`pad-btn ${className} ${held === id ? "on" : ""}`}
      onPointerDown={(e) => {
        e.preventDefault();
        void hold(id);
      }}
      onPointerUp={() => void release()}
      onPointerLeave={() => void release()}
      onPointerCancel={() => void release()}
      aria-label={label}
      title={label}
    >
      {label}
    </button>
  );
}

function Analog({ side, ready }: { side: string; ready: boolean }) {
  const dirs = ["up_left", "up", "up_right", "left", "right", "down_left", "down", "down_right"];
  return (
    <div className="analog">
      {dirs.map((direction) => (
        <button
          key={direction}
          className="pad-btn small"
          disabled={!ready}
          onClick={async () => {
            try {
              await api.padAnalog(side, direction);
            } catch {
              /* handled by the page-level toast for presses */
            }
          }}
        >
          {direction.replace("_", " ")}
        </button>
      ))}
    </div>
  );
}
