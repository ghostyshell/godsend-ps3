// Mock bridge: installs window.__TAURI_INTERNALS__ with PS3 fixture data so
// the UI runs in a plain browser (?mock=1). Never imported by the app.

type Handler = (payload: unknown) => void;

const listeners = new Map<number, { event: string; handler: Handler }>();
let nextCallbackId = 1;

const GB = 1024 * 1024 * 1024;
const MB = 1024 * 1024;

const host = {
  id: "192.168.1.50:21",
  name: "Living room PS3",
  addr: "192.168.1.50",
  ftp_port: 21,
  ftp_user: "anonymous",
  ftp_pass: "",
  http_port: 80,
};

let config = {
  hosts: [host],
  active_host: host.id,
  download_dir: "/Users/demo/Downloads/godsend-ps3",
  rd_api_key: "",
  tb_api_key: "",
  gamesearch_base: "https://game-debrid.link/api",
  covers_base: "https://covers.debrid.in",
  ps3_resources_base: "https://ps3.game-debrid.link",
  aria2_path: "",
  split_bytes: 4095 * MB,
  auto_install_pkg: true,
  pad_release_first: true,
  mcp_port: 8743,
};

const status = {
  host_id: host.id,
  name: host.name,
  addr: host.addr,
  ftp_ok: true,
  webman_ok: true,
  ftp_ms: 12,
  http_ms: 18,
  firmware: "4.90 CEX Evilnat",
  model: "CECH-2501A",
  webman_version: "1.47.50",
  free_bytes: 212 * GB,
  total_bytes: 465 * GB,
  message: "webMAN MOD 1.47.50",
};

const games = [
  {
    id: "iso:/dev_hdd0/PS3ISO/God of War Collection (USA).iso",
    title: "God of War Collection",
    title_id: "BCUS98229",
    system: "ps3",
    kind: "iso",
    category: "disc",
    path: "/dev_hdd0/PS3ISO/God of War Collection (USA).iso",
    size_bytes: 24 * GB,
    has_icon: false,
    icon_path: null,
    version: null,
    device: "hdd0",
  },
  {
    id: "game:/dev_hdd0/game/BLUS30443",
    title: "Demon's Souls",
    title_id: "BLUS30443",
    system: "ps3",
    kind: "game",
    category: "digital",
    path: "/dev_hdd0/game/BLUS30443",
    size_bytes: 8.4 * GB,
    has_icon: true,
    icon_path: "/dev_hdd0/game/BLUS30443/ICON0.PNG",
    version: "01.00",
    device: "hdd0",
  },
  {
    id: "game:/dev_hdd0/game/BLUS31156",
    title: "The Last of Us",
    title_id: "BLUS31156",
    system: "ps3",
    kind: "game",
    category: "digital",
    path: "/dev_hdd0/game/BLUS31156",
    size_bytes: 27 * GB,
    has_icon: true,
    icon_path: "/dev_hdd0/game/BLUS31156/ICON0.PNG",
    version: "01.11",
    device: "hdd0",
  },
  {
    id: "iso:/dev_hdd0/PS3ISO/Gran Turismo 6.iso",
    title: "Gran Turismo 6",
    title_id: "BCES01893",
    system: "ps3",
    kind: "iso",
    category: "disc",
    path: "/dev_hdd0/PS3ISO/Gran Turismo 6.iso",
    size_bytes: 14 * GB,
    has_icon: false,
    icon_path: null,
    version: null,
    device: "hdd0",
  },
  {
    id: "iso:/dev_hdd0/PS2ISO/God of War II.iso",
    title: "God of War II",
    title_id: "SCUS97481",
    system: "ps2",
    kind: "iso",
    category: "disc",
    path: "/dev_hdd0/PS2ISO/God of War II.iso",
    size_bytes: 7.9 * GB,
    has_icon: false,
    icon_path: null,
    version: null,
    device: "hdd0",
  },
  {
    id: "iso:/dev_hdd0/PSXISO/Metal Gear Solid.iso",
    title: "Metal Gear Solid",
    title_id: "SLUS00594",
    system: "ps1",
    kind: "iso",
    category: "disc",
    path: "/dev_hdd0/PSXISO/Metal Gear Solid.iso",
    size_bytes: 720 * MB,
    has_icon: false,
    icon_path: null,
    version: null,
    device: "hdd0",
  },
  {
    id: "game:/dev_hdd0/GAMES/webMAN MOD",
    title: "webMAN MOD",
    title_id: "WMAN00001",
    system: "ps3",
    kind: "homebrew",
    category: "homebrew",
    path: "/dev_hdd0/GAMES/webMAN MOD",
    size_bytes: 15 * MB,
    has_icon: true,
    icon_path: "/dev_hdd0/GAMES/webMAN MOD/ICON0.PNG",
    version: "1.47.50",
    device: "hdd0",
  },
];

const packages = [
  {
    id: "pkg:/dev_hdd0/packages/PS3HEN_v3.3.0.pkg",
    title: "PS3HEN 3.3.0",
    title_id: "HEN000001",
    system: "pkg",
    kind: "pkg",
    category: "package",
    path: "/dev_hdd0/packages/PS3HEN_v3.3.0.pkg",
    size_bytes: 6.2 * MB,
    has_icon: false,
    icon_path: null,
    version: null,
    device: "hdd0",
  },
];

const homebrew = [
  ["webman-mod", "webMAN MOD", "cfw", "aldostools/webMAN-MOD", "1.47.50", "The CFW web server, FTP server and game launcher."],
  ["prepiso", "prepISO", "tool", "aldostools/webMAN-MOD", "1.33", "Mounts games on NTFS, exFAT and network shares."],
  ["ps2-classics", "PS2 Classics Launcher", "tool", "aldostools/webMAN-MOD", "1.47.50", "Launches PS2 ISOs as PS2 Classics."],
  ["ps2config", "PS2 Config", "tool", "aldostools/webMAN-MOD", "1.47.50", "Compatibility database for the PS2 emulator."],
  ["irisman", "IRISMAN", "backup", "aldostools/IRISMAN", "4.91", "All-in-one backup manager."],
  ["apollo", "Apollo Save Tool", "save", "bucanero/apollo-ps3", "v2.4.0", "Save data manager and community saves."],
  ["managunz", "ManaGunZ", "backup", "Zarh/ManaGunZ", "1.41", "Backup manager and game launcher."],
  ["managunz-fm", "ManaGunZ File Manager", "tool", "Zarh/ManaGunZ", "1.41", "Standalone file manager."],
].map(([id, name, category, repo, tag, description]) => ({
  id,
  name,
  description,
  category,
  repo,
  asset: "",
  note: id === "webman-mod" ? "Install this first: the app's remote actions talk to webMAN." : null,
  latest_tag: tag,
  pkg_url: `https://github.com/${repo}/releases/download/${tag}/package.pkg`,
  published_at: "2026-10-10T20:18:40Z",
  size_bytes: 15 * MB,
}));

const searchGames = [
  { id: "god-of-war-collection", slug: "god-of-war-collection", title: "God of War Collection", platform: "ps3", source: "minerva", linkCount: 1, rawgCover: null, cover: null, rating: 4.2, released: "2009-11-17" },
  { id: "demons-souls", slug: "demons-souls", title: "Demon's Souls", platform: "ps3", source: "minerva", linkCount: 2, rawgCover: null, cover: null, rating: 4.5, released: "2009-10-06" },
  { id: "the-last-of-us", slug: "the-last-of-us", title: "The Last of Us", platform: "ps3", source: "minerva", linkCount: 1, rawgCover: null, cover: null, rating: 4.8, released: "2013-06-14" },
];

const iaItems = [
  { id: "ps3-redump-set", title: "PlayStation 3 Redump Collection", files_count: 42, size_bytes: 380 * GB },
  { id: "ps3-homebrew-archive", title: "PS3 Homebrew Archive", files_count: 18, size_bytes: 4.1 * GB },
];

const iaFiles = [
  { name: "God of War Collection (USA).iso", size_bytes: 24 * GB, url: "https://archive.org/x/gow.iso" },
  { name: "Demon's Souls (USA).iso", size_bytes: 8.4 * GB, url: "https://archive.org/x/demons.iso" },
];

const downloadFiles = [
  { name: "God of War Collection (USA).iso", path: "/Users/demo/Downloads/godsend-ps3/God of War Collection (USA).iso", is_dir: false, size_bytes: 24 * GB, kind: "iso" },
  { name: "webMAN_MOD_1.47.50_Installer.pkg", path: "/Users/demo/Downloads/godsend-ps3/webMAN_MOD_1.47.50_Installer.pkg", is_dir: false, size_bytes: 15 * MB, kind: "pkg" },
  { name: "FrogGBA.zip", path: "/Users/demo/Downloads/godsend-ps3/FrogGBA.zip", is_dir: false, size_bytes: 4 * MB, kind: "zip" },
];

const remoteFiles = [
  { name: "PS3ISO", path: "/dev_hdd0/PS3ISO", is_dir: true, size_bytes: 0, kind: "folder" },
  { name: "GAMES", path: "/dev_hdd0/GAMES", is_dir: true, size_bytes: 0, kind: "folder" },
  { name: "packages", path: "/dev_hdd0/packages", is_dir: true, size_bytes: 0, kind: "folder" },
  { name: "PS3HEN_v3.3.0.pkg", path: "/dev_hdd0/packages/PS3HEN_v3.3.0.pkg", is_dir: false, size_bytes: 6.2 * MB, kind: "pkg" },
];

let jobs = [
  {
    id: 2,
    kind: "install",
    label: "webMAN MOD",
    state: "done",
    done_bytes: 100,
    total_bytes: 100,
    message: "uploaded to /dev_hdd0/packages; install requested",
    started_at: Math.floor(Date.now() / 1000) - 600,
    finished_at: Math.floor(Date.now() / 1000) - 590,
  },
  {
    id: 1,
    kind: "download",
    label: "God of War Collection",
    state: "done",
    done_bytes: 100,
    total_bytes: 100,
    message: "downloaded to /Users/demo/Downloads/godsend-ps3",
    started_at: Math.floor(Date.now() / 1000) - 3600,
    finished_at: Math.floor(Date.now() / 1000) - 3590,
  },
];

const doctor = [
  { name: "Console configured", ok: true, detail: "Living room PS3 at 192.168.1.50" },
  { name: "FTP reachable", ok: true, detail: "12 ms" },
  { name: "webMAN MOD", ok: true, detail: "v1.47.50" },
  { name: "Firmware", ok: true, detail: "4.90 CEX Evilnat" },
  { name: "Internal HDD space", ok: true, detail: "212.0 GB free" },
  { name: "Download folder", ok: true, detail: "/Users/demo/Downloads/godsend-ps3" },
  { name: "aria2c", ok: true, detail: "/opt/homebrew/bin/aria2c" },
  { name: "Debrid", ok: false, detail: "optional: keys unlock debrid download links" },
  { name: "MCP server", ok: true, detail: "http://127.0.0.1:8743/mcp, started with the app" },
  { name: "ps3-resources", ok: true, detail: "https://ps3.game-debrid.link (title ids, updates, disc keys)" },
];

async function invoke(cmd: string, args: Record<string, any> = {}): Promise<unknown> {
  switch (cmd) {
    case "snapshot":
      return { hosts: config.hosts, active: config.active_host, config, doctor, statuses: [status], jobs };
    case "probe_host":
    case "probe_all":
      return cmd === "probe_all" ? [status] : status;
    case "discover":
      return [
        { id: "192.168.1.51:21", name: "PS3 192.168.1.51", addr: "192.168.1.51", ftp_port: 21, ftp_user: "anonymous", ftp_pass: "", http_port: 80 },
      ];
    case "host_add": {
      const incoming = args.host as any;
      config = { ...config, hosts: [...config.hosts.filter((h) => h.id !== incoming.id), incoming] };
      return config;
    }
    case "host_remove":
      config = { ...config, hosts: config.hosts.filter((h) => h.id !== args.id) };
      return config;
    case "host_set_active":
      config = { ...config, active_host: args.id };
      return config;
    case "library_scan":
      return { games, packages, roots: ["/dev_hdd0/game", "/dev_hdd0/GAMES", "/dev_hdd0/PS3ISO"] };
    case "icon":
      return iconDataUrl(String(args.path));
    case "mount_game":
    case "install_remote":
    case "pad_press":
    case "pad_hold":
    case "pad_release":
    case "pad_combo":
    case "pad_analog":
    case "pad_macro":
    case "system_action":
    case "library_delete":
    case "config_set":
      if (cmd === "config_set") config = { ...config, ...(args.patch as object) };
      return cmd === "config_set" ? config : null;
    case "homebrew_catalog":
      return homebrew;
    case "homebrew_install":
    case "store_download":
    case "update_install":
    case "download_push":
    case "install_path":
    case "fm_upload":
    case "fm_download":
    case "convert_run": {
      const id = Math.max(0, ...jobs.map((j) => j.id)) + 1;
      jobs = [
        {
          id,
          kind: cmd,
          label: String(args.id ?? args.name ?? args.path ?? args.titleId ?? args.kind ?? "job"),
          state: "done",
          done_bytes: 100,
          total_bytes: 100,
          message: "done (mock)",
          started_at: Math.floor(Date.now() / 1000),
          finished_at: Math.floor(Date.now() / 1000),
        },
        ...jobs,
      ];
      return id;
    }
    case "store_search":
      return args.source === "ia"
        ? { kind: "ia", items: iaItems }
        : { kind: "gamesearch", games: searchGames };
    case "store_files":
      return iaFiles;
    case "downloads_list":
      return { dir: config.download_dir, files: downloadFiles };
    case "download_open":
    case "download_delete":
      return null;
    case "fm_local":
      return downloadFiles;
    case "fm_remote":
      return remoteFiles;
    case "fm_mkdir":
    case "fm_mkdir_remote":
      return "/Users/demo/Downloads/godsend-ps3/new-folder";
    case "fm_delete":
      return null;
    case "convert_inspect": {
      const path = String(args.path);
      const name = path.split("/").pop() ?? path;
      return {
        path,
        name,
        size_bytes: 24 * GB,
        kind: name.endsWith(".iso") ? "iso" : name.endsWith(".pkg") ? "pkg" : "folder",
        system: "ps3",
        title_id: "BCUS98229",
        encrypted: name.endsWith(".iso"),
        parts: 0,
      };
    }
    case "updates_list":
      return [
        { titleId: args.titleId, name: "God of War Collection", version: "01.01", sizeBytes: 180 * MB, pkgUrl: "https://x/update.pkg", firmware: "" },
        { titleId: args.titleId, name: "God of War Collection", version: "01.00", sizeBytes: 175 * MB, pkgUrl: "https://x/update0.pkg", firmware: "" },
      ];
    case "title_lookup":
      return [{ titleId: args.query, name: "God of War Collection" }];
    case "config_get":
      return config;
    case "doctor_run":
      return doctor;
    case "jobs":
      return jobs;
    case "job_cancel":
      return true;
    case "jobs_clear":
      jobs = jobs.filter((j) => j.state === "running");
      return null;
    case "reveal":
    case "open_data_dir":
      return null;
    case "plugin:dialog|open": {
      const options = (args.options ?? {}) as { directory?: boolean; filters?: { extensions: string[] }[] };
      if (options.directory) return "/Users/demo/Downloads/godsend-ps3";
      const ext = options.filters?.[0]?.extensions?.[0] ?? "iso";
      return `/Users/demo/Downloads/godsend-ps3/God of War Collection.${ext}`;
    }
    case "plugin:dialog|save":
      return "/Users/demo/Downloads/godsend-ps3/output.iso";
    case "plugin:event|listen": {
      const id = nextCallbackId++;
      listeners.set(id, { event: args.event, handler: args.handler });
      return id;
    }
    case "plugin:event|unlisten":
      listeners.delete(args.eventId);
      return null;
    default:
      console.warn("[mock] unhandled invoke:", cmd, args);
      return null;
  }
}

function iconDataUrl(seed: string): string {
  const canvas = document.createElement("canvas");
  canvas.width = 288;
  canvas.height = 160;
  const ctx = canvas.getContext("2d")!;
  let hash = 0;
  for (const ch of seed) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  const hue = Math.abs(hash) % 360;
  const grad = ctx.createLinearGradient(0, 0, 288, 160);
  grad.addColorStop(0, `hsl(${hue} 58% 34%)`);
  grad.addColorStop(1, `hsl(${(hue + 45) % 360} 62% 16%)`);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 288, 160);
  const glow = ctx.createRadialGradient(230, 10, 10, 230, 10, 160);
  glow.addColorStop(0, "rgba(255,255,255,0.22)");
  glow.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, 288, 160);
  ctx.fillStyle = "rgba(255,255,255,0.95)";
  ctx.font = "700 22px 'Chakra Petch', sans-serif";
  ctx.textBaseline = "bottom";
  const title = seed.split("/").slice(-2, -1)[0] ?? "PS3";
  ctx.fillText(title.slice(0, 24), 16, 128);
  return canvas.toDataURL("image/png");
}

(window as unknown as Record<string, any>).__TAURI_INTERNALS__ = {
  invoke,
  transformCallback(cb: Handler) {
    const id = nextCallbackId++;
    listeners.set(id, { event: "__callback__", handler: cb });
    return id;
  },
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main" } },
  plugins: {},
};

console.info("[mock] GODsend PS3 fixture bridge installed");
export {}; // make this a module so dynamic import type-checks
