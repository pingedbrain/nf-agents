# SPDX-License-Identifier: Apache-2.0
"""Scripted MCP session against nf-lineage-mcp for the local demo (demo/run.sh).

Speaks JSON-RPC over stdin/stdout, prints a short human summary of each step on stderr and
writes the raw exchange to <workdir>/transcript.jsonl. Python >= 3.9, stdlib only.
Exit 0 when every answer is the expected one, 1 otherwise (the message names the step).
"""
import argparse
import json
import os
import sys

TAMPER_LINE = "edited by the nf-agents demo to show a checksum mismatch\n"


class TourError(Exception):
    pass


class Session:
    def __init__(self, rx, tx, transcript=None):
        self.rx, self.tx, self.transcript = rx, tx, transcript
        self.last_id = 0
        self.step = "initialize"

    def _log(self, line):
        if self.transcript:
            self.transcript.write(line.rstrip("\n") + "\n")

    def _send(self, msg):
        line = json.dumps(msg, ensure_ascii=False)
        self.tx.write(line + "\n")
        self.tx.flush()
        self._log(line)

    def notify(self, method):
        self._send({"jsonrpc": "2.0", "method": method})

    def request(self, method, params):
        self.last_id += 1
        self._send({"jsonrpc": "2.0", "id": self.last_id, "method": method, "params": params})
        while True:
            line = self.rx.readline()
            if not line:
                raise TourError(f"{self.step}: server closed the connection")
            try:
                msg = json.loads(line)
            except ValueError:
                continue  # not a JSON-RPC message
            if not isinstance(msg, dict) or msg.get("id") != self.last_id:
                continue
            self._log(line)
            if "error" in msg:
                raise TourError(f"{self.step}: {msg['error']}")
            return msg.get("result", {})

    def tool(self, name, args):
        self.step = name
        result = self.request("tools/call", {"name": name, "arguments": args})
        body = "".join(c.get("text", "") for c in result.get("content", []))
        if result.get("isError"):
            raise TourError(f"{name}: {body}")
        try:
            return json.loads(body)
        except ValueError:
            raise TourError(f"{name}: expected JSON, got {body[:200]!r}") from None


def expect(ok, message):
    if not ok:
        raise TourError(message)


def run_tour(s, workdir, say):
    runs = s.tool("list_runs", {})
    expect(len(runs) == 2, f"list_runs: expected 2 runs, got {len(runs)}")
    b, a = runs  # newest first
    say(f"1. list_runs: run A {a['run_name']} ({a['lid']}), run B {b['run_name']} ({b['lid']})")

    changes = s.tool("diff", {"lid_a": a["lid"], "lid_b": b["lid"]})["changes"]
    want = [{"path": "params.greeting", "a": "hello", "b": "bonjour"}]
    expect(changes == want, "diff: expected only params.greeting, got "
           + ", ".join(f"{c['path']}: {c['a']!r} -> {c['b']!r}" for c in changes))
    say('2. diff A -> B: params.greeting: "hello" → "bonjour" (nothing else changed)')

    agents = s.tool("find", {"terms": ["type=AgentRun", f"workflowRun={b['lid']}"]})
    expect(len(agents) == 1, f"find: expected 1 AgentRun in run B, got {agents}")
    spec = s.tool("view", {"lid": agents[0]})["spec"]
    verdict = s.tool("view", {"lid": agents[0] + "#output"})[0]["value"]
    expect(spec["model"] == "openai/gpt-5-mini" and verdict["answer"] == "no",
           f"view: unexpected AI step {spec['model']} -> {verdict}")
    say(f'3. AI step {spec["name"]}: model {spec["model"]}, answer "{verdict["answer"]}" '
        f'(confidence {verdict["confidence"]}) to "any lowercase letters?"')

    nodes = s.tool("provenance", {"lid": b["lid"] + "/summary.txt"})["nodes"]
    tasks = [n["name"] for n in nodes if n["kind"] == "TaskRun"]
    seeds = [i["path"] for n in nodes for i in n["inputs"] if i["type"] == "external"]
    expect(tasks == ["SUMMARIZE", "TRANSFORM", "GENERATE"] and len(seeds) == 1
           and seeds[0].endswith("/seed.txt"), f"provenance: unexpected chain {tasks} {seeds}")
    say("4. provenance of run B summary.txt: SUMMARIZE ← TRANSFORM ← GENERATE ← seed.txt")

    target = {"lid": a["lid"] + "/summary.txt"}
    report = s.tool("check", target)
    expect(report["status"] == "valid", f"check: expected valid, got {report['status']}")
    say(f"5. check run A summary.txt: Valid ({report['path']})")
    path = os.path.realpath(report["path"])
    expect(path.startswith(os.path.realpath(workdir) + os.sep),
           f"check: {path} is outside the demo workdir; not editing it")
    with open(path, "a") as f:
        f.write(TAMPER_LINE)
    status = s.tool("check", target)["status"]
    expect(status == "mismatch", f"check: expected mismatch after the edit, got {status}")
    say("6. check again after appending a line to it: Mismatch (the file no longer matches its lineage)")


def main(argv=None, rx=sys.stdin, tx=sys.stdout, err=sys.stderr):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workdir", required=True, help="demo workdir as mounted for the server")
    args = parser.parse_args(argv)

    def say(message):
        print(message, file=err, flush=True)

    with open(os.path.join(args.workdir, "transcript.jsonl"), "w") as transcript:
        s = Session(rx, tx, transcript)
        try:
            s.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                     "clientInfo": {"name": "nf-agents-demo", "version": "0"}})
            s.notify("notifications/initialized")
            run_tour(s, args.workdir, say)
        except TourError as e:
            say(f"tour failed: {e}")
            return 1
        except (KeyError, IndexError, TypeError, ValueError, AttributeError) as e:
            say(f"tour failed: {s.step}: unexpected answer shape ({e!r})")
            return 1
    say("tour passed: 6/6 answers as expected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
