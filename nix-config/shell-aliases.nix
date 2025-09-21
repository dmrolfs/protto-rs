# DMR: Externalized shell aliases configuration
{
  # Modern CLI replacements
  ls = "eza --color=always --group-directories-first";
  ll = "eza -la --color=always --group-directories-first";
  l = "eza -la --color=always --group-directories-first";
  tree = "eza --tree --color=always";
  cat = "bat --paging=never";
  grep = "rg";
  find = "fd";
  ps = "procs";
  du = "dust";
  ping = "gping";

  # Git shortcuts
  g = "git";
  gs = "git status";
  ga = "git add";
  gc = "git commit";
  gp = "git push";
  gl = "git log --oneline";
  gd = "git diff";
  gb = "git branch";
  gco = "git checkout";

  # Cargo shortcuts
  cr = "cargo run";
  ct = "cargo test";
  cb = "cargo build";
  cc = "cargo check";
  cf = "cargo fmt";
  clippy = "cargo clippy";
  cw = "cargo watch";

  # Container shortcuts
  pc = "podman-compose";
  pcup = "podman-compose up -d";
  pcdown = "podman-compose down";
  pcps = "podman-compose ps";
  pclogs = "podman-compose logs -f";

  # System shortcuts
  top = "htop";
  df = "dust";
  ps-tree = "procs --tree";

  # Network shortcuts
  myip = "curl -s http://checkip.amazonaws.com";
  ports = "if command -v ss >/dev/null 2>&1; then ss -tuln; elif command -v netstat >/dev/null 2>&1; then netstat -tuln; else echo 'Neither ss nor netstat available'; fi";

  # Development shortcuts
  json = "jq";

  # File operations
  backup = "cp -r";
  mkdir = "mkdir -p";

  # DMR: Log analysis shortcuts
  logs = "lnav";
  logf = "lnav -f"; # Follow mode
  logwatch = "lnav -t"; # Tail mode
}
