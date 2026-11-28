# SPDX-License-Identifier: Apache-2.0
"""Tests for demo/tour.py (canned MCP server, no Docker) and demo/run.sh's early checks.

Run: python3 -I -m unittest discover -s demo -p 'test_*.py'
"""
import io
import json
import os
import subprocess
import tempfile
import unittest

import tour

A = "lid://" + "a" * 32
B = "lid://" + "b" * 32
AGENT = "lid://" + "c" * 32
HERE = os.path.dirname(os.path.abspath(__file__))


def text(value):
    return {"content": [{"type": "text", "text": json.dumps(value)}], "isError": False}


class FakeServer:
    """Both pipes of an MCP server: `write` takes a request, `readline` returns its answer."""

    def __init__(self, workdir):
        self.summary_a = os.path.join(workdir, "results-hello", "summary.txt")
        os.makedirs(os.path.dirname(self.summary_a))
        with open(self.summary_a, "w") as f:
            f.write("lines: 3\n")
        self.results = {
            "list_runs": [{"run_name": "second", "lid": B}, {"run_name": "first", "lid": A}],
            "diff": {"a": A, "b": B, "changes": [{"path": "params.greeting", "a": "hello", "b": "bonjour"}]},
            "find": [AGENT],
            "view": {"kind": "AgentRun", "spec": {"name": "QC_NOTE", "model": "openai/gpt-5-mini"}},
            "view#output": [{"type": "Record", "name": "verdict", "value": {"answer": "no", "confidence": 0.9}}],
            "provenance": {"target": B + "/summary.txt", "nodes": [
                {"kind": "FileOutput", "name": "/demo/results-bonjour/summary.txt", "inputs": []},
                {"kind": "TaskRun", "name": "SUMMARIZE", "inputs": []},
                {"kind": "TaskRun", "name": "TRANSFORM", "inputs": []},
                {"kind": "TaskRun", "name": "GENERATE", "inputs": [
                    {"type": "external", "name": "seed", "path": "file:///src/data/seed.txt"},
                    {"type": "value", "name": "greeting", "value": "bonjour"}]},
            ]},
        }
        self.overrides = {}
        self.eof_on = None
        self.queue = []

    def check(self):
        with open(self.summary_a) as f:
            intact = f.read() == "lines: 3\n"
        return {"lid": A + "/summary.txt", "path": self.summary_a, "status": "valid" if intact else "mismatch"}

    def write(self, line):
        msg = json.loads(line)
        if "id" not in msg:
            return
        if msg["method"] == "initialize":
            result = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}}
        else:
            name = msg["params"]["name"]
            if name == self.eof_on:
                return
            key = name + ("#output" if msg["params"]["arguments"].get("lid", "").endswith("#output") else "")
            result = self.overrides.get(key) or text(self.check() if name == "check" else self.results[key])
        # A log line and an unrelated id come first: the client must skip both.
        self.queue += ["not json\n", json.dumps({"jsonrpc": "2.0", "id": 999, "result": {}}) + "\n"]
        self.queue.append(json.dumps({"jsonrpc": "2.0", "id": msg["id"], "result": result}) + "\n")

    def flush(self):
        pass

    def readline(self):
        return self.queue.pop(0) if self.queue else ""


class TourTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.workdir = tmp.name
        self.server = FakeServer(self.workdir)

    def run_tour(self):
        err = io.StringIO()
        code = tour.main(["--workdir", self.workdir], rx=self.server, tx=self.server, err=err)
        return code, err.getvalue()

    def test_happy_path_prints_answers_and_exits_0(self):
        code, out = self.run_tour()
        self.assertEqual(code, 0, out)
        for want in ['params.greeting: "hello" → "bonjour"', "openai/gpt-5-mini", 'answer "no"',
                     "SUMMARIZE ← TRANSFORM ← GENERATE ← seed.txt", "Valid", "Mismatch"]:
            self.assertIn(want, out)
        with open(self.server.summary_a) as f:
            self.assertIn("edited by the baqueano demo", f.read())
        with open(os.path.join(self.workdir, "transcript.jsonl")) as f:
            self.assertIn('"method": "initialize"', f.read())

    def test_unexpected_diff_fails(self):
        extra = {"path": "tasks.GENERATE.script", "a": "x", "b": "y"}
        self.server.results["diff"]["changes"].append(extra)
        code, out = self.run_tour()
        self.assertEqual(code, 1)
        self.assertIn("diff", out)
        self.assertIn("tasks.GENERATE.script", out)

    def test_tool_error_fails_with_server_message(self):
        self.server.overrides["provenance"] = {
            "content": [{"type": "text", "text": "not_found: no record for lid://b"}], "isError": True}
        code, out = self.run_tour()
        self.assertEqual(code, 1)
        self.assertIn("provenance: not_found: no record for lid://b", out)

    def test_server_eof_fails_cleanly(self):
        self.server.eof_on = "diff"
        code, out = self.run_tour()
        self.assertEqual(code, 1)
        self.assertIn("server closed", out)
        self.assertNotIn("Traceback", out)

    def test_unexpected_answer_shape_names_the_step(self):
        self.server.results["find"] = {"oops": 1}
        code, out = self.run_tour()
        self.assertEqual(code, 1)
        self.assertIn("find", out)
        self.assertNotIn("Traceback", out)

    def test_never_edits_files_outside_the_workdir(self):
        outside = tempfile.NamedTemporaryFile("w", delete=False)
        outside.write("lines: 3\n")
        outside.close()
        self.addCleanup(os.unlink, outside.name)
        self.server.summary_a = outside.name
        code, out = self.run_tour()
        self.assertEqual(code, 1)
        self.assertIn("outside", out)
        with open(outside.name) as f:
            self.assertEqual(f.read(), "lines: 3\n")


class RunScriptTests(unittest.TestCase):
    """demo/run.sh checks that happen before any container starts."""

    def run_script(self, workdir, **env):
        stub = tempfile.TemporaryDirectory()
        self.addCleanup(stub.cleanup)
        docker = os.path.join(stub.name, "docker")
        with open(docker, "w") as f:
            f.write("#!/bin/sh\nexit 1\n")  # a daemon that is down
        os.chmod(docker, 0o755)
        env = dict(PATH=stub.name + os.pathsep + os.environ["PATH"], DEMO_WORKDIR=workdir, **env)
        return subprocess.run(["bash", os.path.join(HERE, "run.sh")], env=env,
                              capture_output=True, text=True, timeout=30)

    def workdir(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        return tmp.name

    def test_refuses_foreign_workdir(self):
        workdir = self.workdir()
        keep = os.path.join(workdir, "keep.txt")
        open(keep, "w").close()
        r = self.run_script(workdir)
        self.assertEqual(r.returncode, 1)
        self.assertIn("refusing to delete", r.stderr)
        self.assertTrue(os.path.exists(keep))

    def test_fails_fast_without_docker(self):
        r = self.run_script(self.workdir())
        self.assertEqual(r.returncode, 1)
        self.assertIn("docker is required", r.stderr)

    def test_record_requires_upstream(self):
        r = self.run_script(self.workdir(), DEMO_RECORD="1")
        self.assertEqual(r.returncode, 1)
        self.assertIn("UPSTREAM", r.stderr)


if __name__ == "__main__":
    unittest.main()
