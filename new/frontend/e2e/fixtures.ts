import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import {
  closeSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test as base, type TestInfo } from "@playwright/test";

const root = fileURLToPath(new URL("../../", import.meta.url));
const frontend = join(root, "frontend");
const backendBinary = join(root, "backend", "target", "debug", "food-backend");
const idpBinary = join(root, "backend", "target", "debug", "dev_idp");
const oidcClientId = "food-e2e";
const oidcClientSecret = "food-e2e-secret";
const sessionKey = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

type RunningProcess = {
  child: ChildProcess;
  log: string;
};

type WorkerStack = {
  appUrl: string;
  backendUrl: string;
  backendPort: number;
  idpUrl: string;
  directory: string;
  vite: RunningProcess;
  idp: RunningProcess;
};

type TestFixtures = {
  backend: void;
};

type WorkerFixtures = {
  stack: WorkerStack;
};

export const test = base.extend<TestFixtures, WorkerFixtures>({
  stack: [
    async ({ browserName: _browserName }, provide, workerInfo) => {
      const directory = mkdtempSync(
        join(tmpdir(), `food-e2e-worker-${workerInfo.workerIndex}-`),
      );
      const [appPort, backendPort, idpPort] = await freePorts(3);
      const appUrl = `http://127.0.0.1:${appPort}`;
      const backendUrl = `http://127.0.0.1:${backendPort}`;
      const idpUrl = `http://127.0.0.1:${idpPort}`;
      const idp = startProcess(
        idpBinary,
        [],
        root,
        join(directory, "idp.log"),
        {
          APP_URL: appUrl,
          OIDC_ISSUER: idpUrl,
          OIDC_CLIENT_ID: oidcClientId,
          OIDC_CLIENT_SECRET: oidcClientSecret,
          HOST: "127.0.0.1",
          PORT: String(idpPort),
        },
      );
      const vite = startProcess(
        "pnpm",
        [
          "run",
          process.env.E2E_PREVIEW === "1" ? "preview" : "dev",
          "--port",
          String(appPort),
        ],
        frontend,
        join(directory, "vite.log"),
        {
          VITE_PORT: String(appPort),
          BACKEND_URL: backendUrl,
          VITE_CACHE_DIR: join(directory, "vite-cache"),
        },
      );
      const stack = {
        appUrl,
        backendUrl,
        backendPort,
        idpUrl,
        directory,
        vite,
        idp,
      };

      try {
        await Promise.all([
          waitForHttp(`${idpUrl}/.well-known/openid-configuration`, idp),
          waitForHttp(appUrl, vite),
        ]);
        await provide(stack);
      } finally {
        await Promise.all([stopProcess(vite), stopProcess(idp)]);
        rmSync(directory, { recursive: true, force: true });
      }
    },
    { scope: "worker" },
  ],

  backend: [
    async ({ stack }, provide, testInfo) => {
      const adminUrl = testDatabaseUrl();
      const database = databaseName(testInfo.workerIndex);
      const databaseUrl = databaseUrlFor(adminUrl, database);
      const log = testInfo.outputPath("backend.log");
      mkdirSync(dirname(log), { recursive: true });
      createDatabase(adminUrl, database);
      const backend = startProcess(backendBinary, [], root, log, {
        DATABASE_URL: databaseUrl,
        APP_URL: stack.appUrl,
        OIDC_ISSUER: stack.idpUrl,
        OIDC_CLIENT_ID: oidcClientId,
        OIDC_CLIENT_SECRET: oidcClientSecret,
        SESSION_ENCRYPTION_KEY: sessionKey,
        ALLOW_INSECURE_OIDC: "1",
        HOST: "127.0.0.1",
        PORT: String(stack.backendPort),
      });

      try {
        await waitForHttp(`${stack.backendUrl}/health`, backend);
        execFileSync(
          join(root, "backend", "target", "debug", "seed-test-catalog"),
          [],
          {
            env: { ...process.env, DATABASE_URL: databaseUrl },
            stdio: "inherit",
          },
        );
        await provide();
      } finally {
        await stopProcess(backend);
        dropDatabase(adminUrl, database);
        if (testInfo.status !== testInfo.expectedStatus) {
          await attachLog(testInfo, "backend", log);
          await attachLog(testInfo, "Vite", stack.vite.log);
          await attachLog(testInfo, "dev IdP", stack.idp.log);
        }
      }
    },
    { auto: true },
  ],

  baseURL: async ({ stack, backend: _backend }, provide) => {
    await provide(stack.appUrl);
  },
});

export { expect };

async function freePorts(count: number) {
  const reservations = await Promise.all(
    Array.from({ length: count }, () => reservePort()),
  );
  const ports = reservations.map(({ port }) => port);
  await Promise.all(
    reservations.map(
      ({ server }) =>
        new Promise<void>((resolve, reject) =>
          server.close((error) => (error ? reject(error) : resolve())),
        ),
    ),
  );
  return ports;
}

function reservePort() {
  return new Promise<{
    server: ReturnType<typeof createServer>;
    port: number;
  }>((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        reject(new Error("Could not reserve a test port"));
        return;
      }
      resolve({ server, port: address.port });
    });
  });
}

function testDatabaseUrl() {
  const value = process.env.TEST_DATABASE_URL ?? process.env.DATABASE_URL;
  if (!value) {
    throw new Error(
      "TEST_DATABASE_URL or DATABASE_URL must point to a PostgreSQL database whose role can create databases",
    );
  }
  return value;
}

function databaseName(workerIndex: number) {
  const nonce = Math.random().toString(36).slice(2, 10);
  return `food_e2e_${process.pid}_${workerIndex}_${nonce}`;
}

function databaseUrlFor(adminUrl: string, database: string) {
  const url = new URL(adminUrl);
  url.pathname = `/${database}`;
  return url.toString();
}

function createDatabase(adminUrl: string, database: string) {
  execFileSync("createdb", ["--maintenance-db", adminUrl, database], {
    stdio: "inherit",
  });
}

function dropDatabase(adminUrl: string, database: string) {
  execFileSync(
    "dropdb",
    ["--if-exists", "--force", "--maintenance-db", adminUrl, database],
    { stdio: "inherit" },
  );
}

function startProcess(
  command: string,
  args: string[],
  cwd: string,
  log: string,
  environment: Record<string, string>,
): RunningProcess {
  mkdirSync(dirname(log), { recursive: true });
  const output = openSync(log, "a");
  const child = spawn(command, args, {
    cwd,
    detached: true,
    env: { ...process.env, ...environment },
    stdio: ["ignore", output, output],
  });
  closeSync(output);
  return { child, log };
}

async function stopProcess(processToStop: RunningProcess) {
  const child = processToStop.child;
  if (child.exitCode !== null || child.signalCode !== null || !child.pid)
    return;

  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {
    return;
  }
  await Promise.race([waitForExit(child), delay(5_000)]);
  if (child.exitCode === null && child.signalCode === null) {
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch {
      return;
    }
    await waitForExit(child);
  }
}

function waitForExit(child: ChildProcess) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve();
  }
  return new Promise<void>((resolve) => child.once("exit", () => resolve()));
}

async function waitForHttp(url: string, running: RunningProcess) {
  for (let attempt = 0; attempt < 300; attempt += 1) {
    if (running.child.exitCode !== null || running.child.signalCode !== null) {
      throw new Error(
        `Process stopped before ${url} was ready:\n${readLog(running.log)}`,
      );
    }
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      await delay(100);
    }
  }
  throw new Error(`Timed out waiting for ${url}:\n${readLog(running.log)}`);
}

function delay(milliseconds: number) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function readLog(path: string) {
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

async function attachLog(testInfo: TestInfo, name: string, path: string) {
  if (!existsSync(path)) return;
  await testInfo.attach(`${name} log`, {
    body: readFileSync(path),
    contentType: "text/plain",
  });
}
