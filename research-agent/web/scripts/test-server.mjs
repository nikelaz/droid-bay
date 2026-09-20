import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { spawn } from "node:child_process";

const directory = await mkdtemp(join(tmpdir(), "research-ui-test-"));
const config = join(directory, "config.json");
await writeFile(config, JSON.stringify({ providers: ["mock"] }));
const child = spawn(
  resolve("../target/debug/research-agent"),
  [
    "--port",
    "18081",
    "--config",
    config,
    "--db",
    join(directory, "test.db"),
    "--web",
    resolve("out"),
  ],
  { stdio: "inherit" },
);
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => child.kill("SIGTERM"));
child.on("error", async (error) => {
  console.error(error);
  await rm(directory, { recursive: true, force: true });
  process.exit(1);
});
child.on("exit", async (code) => {
  await rm(directory, { recursive: true, force: true });
  process.exit(code ?? 0);
});
