// Public Ketbook lives in ketbook/. HonKit resolves relative output against the book dir.
const { spawnSync } = require("child_process");
const path = require("path");

const root = path.resolve(__dirname, "..");
const book = path.join(root, "ketbook");
const out = path.join(root, "frontend", "ketbook");
const r = spawnSync("npx", ["honkit", "build", book, out], {
  cwd: root,
  stdio: "inherit",
  shell: true,
});
process.exit(r.status === null ? 1 : r.status);
