"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { OpenFang } = require("../index.js");

test("agents.upload sends raw body with preserved auth and filename headers", async () => {
  const client = new OpenFang("http://localhost:4200", {
    headers: { Authorization: "Bearer secret-token" },
  });
  const blob = new Blob(["hello world"], { type: "text/plain" });

  let captured = null;
  global.fetch = async function (url, init) {
    captured = { url, init };
    return {
      ok: true,
      status: 201,
      async json() {
        return {
          file_id: "file-1",
          filename: "notes.txt",
          content_type: "text/plain",
        };
      },
    };
  };

  const result = await client.agents.upload("agent-1", blob, "notes.txt");

  assert.equal(result.file_id, "file-1");
  assert.equal(captured.url, "http://localhost:4200/api/agents/agent-1/upload");
  assert.equal(captured.init.method, "POST");
  assert.equal(captured.init.body, blob);
  assert.equal(captured.init.headers.Authorization, "Bearer secret-token");
  assert.equal(captured.init.headers["Content-Type"], "text/plain");
  assert.equal(captured.init.headers["X-Filename"], "notes.txt");
});
