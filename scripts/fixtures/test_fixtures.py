"""Offline tests for the fixture helpers (stdlib unittest, Python >= 3.9).

Run: python3 -I -m unittest discover -s scripts/fixtures -p 'test_*.py'
"""

import json
import os
import shutil
import subprocess
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from pathlib import Path

import check
import export
import ids
import llm_stub
import restore_mtimes
import shape

HERE = Path(__file__).resolve().parent
ROOT = "/nf-agents-fixture"
R1, R2, T1, T2 = "a" * 32, "b" * 32, "c" * 32, "d" * 32
TARGET_NS = 1_700_000_000_123_456_789


def rec(kind, envelope, **fields):
    if envelope == "spec":
        return {"version": "lineage/v1beta1", "kind": kind, "spec": fields}
    return {"version": "lineage/v1beta1", "kind": kind, **fields}


def put(store, lid_path, obj):
    p = store / lid_path / ".data.json"
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj))


def live_tree(base, envelope="spec"):
    """A tiny live run tree as Nextflow leaves it: store, task dirs, published symlinks."""
    root = base / "root"
    store = root / ".lineage"
    store.mkdir(parents=True)
    (root / "nextflow.config").write_text("lineage.enabled = true\n")
    for run, task, g, wd in ((R1, T1, "hello", "work/cc/cc"), (R2, T2, "bonjour", "work/dd/dd")):
        params = [{"type": "String", "name": "greeting", "value": g}]
        put(store, run, rec("WorkflowRun", envelope, name="run_" + g, params=params,
                            config={"poolSize": 2}))
        put(store, task, rec("TaskRun", envelope, name="SUMMARIZE", workflowRun="lid://" + run))
        put(store, run + "#output", rec("WorkflowOutput", envelope, workflowRun="lid://" + run))
        tdir = root / wd
        tdir.mkdir(parents=True)
        for n in (".command.sh", ".exitcode"):
            (tdir / n).write_text("x")
        (tdir / "summary.txt").write_text("lines: 3\n")
        os.utime(tdir / "summary.txt", ns=(TARGET_NS, TARGET_NS))
        put(store, task + "/summary.txt", rec("FileOutput", envelope, path=f"{ROOT}/{wd}/summary.txt",
                                              taskRun="lid://" + task, workflowRun="lid://" + run))
        for rel in ("summary.txt", "upper/upper.txt"):
            pub = root / f"results-{g}" / rel
            pub.parent.mkdir(parents=True, exist_ok=True)
            os.symlink(tdir / "summary.txt", pub)
            put(store, f"{run}/{rel}", rec("FileOutput", envelope, path=f"{ROOT}/results-{g}/{rel}",
                                           taskRun=None, workflowRun="lid://" + run))
    (store / ".history").mkdir()
    (store / ".history" / R1).write_text("history line\n")
    return root


def agent_tree(base):
    """A live agent run tree: one WorkflowRun, AgentRuns with tools / outputSchema, no files."""
    root = base / "root"
    store = root / ".lineage"
    store.mkdir(parents=True)
    (root / "nextflow.config").write_text("lineage.enabled = true\n")
    put(store, R1, rec("WorkflowRun", "spec", name="run_a", params=[], config={"poolSize": 2}))
    for lid, name, extra in ((T1, "qa", {}), (T2, "qa_structured", {"outputSchema": {"type": "object"}}),
                             ("e" * 32, "shouty", {"tools": ["uppercase"]})):
        put(store, lid, rec("AgentRun", "spec", name=name, runner="langchain4j", workflowRun="lid://" + R1,
                            **{"tools": None, "outputSchema": None, **extra}))
    return root


def version_dir(base, kind="core"):
    """A valid exported version dir with a complete (dummy) oracle and shape.txt."""
    out = base / "v"
    tree, cases = (live_tree, check.CORE_CASES) if kind == "core" else (agent_tree, check.AGENT_CASES)
    export.export(tree(base), out, kind=kind, version="26.10.7", image="img", recorded_root=ROOT)
    oracle = out / "oracle"
    oracle.mkdir()
    for case in cases:
        (oracle / f"{case}.args").write_text('["lineage", "list"]\n')
        (oracle / f"{case}.out").write_text("")
        (oracle / f"{case}.err").write_text("")
        (oracle / f"{case}.exit").write_text("0\n")
    (oracle / "ids.json").write_text(json.dumps({"RUN1": "lid://" + R1, "RUN2": "lid://" + R2}))
    (out / "shape.txt").write_text(shape.shape(out))
    return out


def manifest(d):
    return json.loads((d / "manifest.json").read_text())


def write_manifest(d, m):
    (d / "manifest.json").write_text(json.dumps(m))


class Tmp(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.base = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()


class CasesTests(Tmp):
    def test_unsafe_arg_rejected(self):
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        marker = self.base / "ran"
        fake = bin_dir / "nextflow"
        fake.write_text(f"#!/bin/sh\ntouch {marker}\n")
        fake.chmod(0o755)
        oracle = self.base / "oracle"
        oracle.mkdir()
        for bad in ("a b", "x'y", 'x"y', "$(id)"):
            r = subprocess.run(
                ["bash", "-c", 'source "$1"; cap view-bad lineage view "$2"', "_", str(HERE / "lib.sh"), bad],
                env={"PATH": f"{bin_dir}:/usr/bin:/bin", "ORACLE": str(oracle)},
                capture_output=True, text=True)
            self.assertNotEqual(r.returncode, 0, bad)
            self.assertIn("unsafe", r.stderr)
            self.assertFalse(marker.exists(), bad)
            self.assertEqual(list(oracle.iterdir()), [])


class ExportTests(Tmp):
    def test_symlink_dereferenced(self):
        out = version_dir(self.base)
        f = out / "results-hello" / "summary.txt"
        self.assertFalse(f.is_symlink())
        self.assertEqual(f.read_text(), "lines: 3\n")
        self.assertEqual(f.stat().st_mtime_ns, TARGET_NS)
        m = manifest(out)
        self.assertEqual([m[k] for k in ("format", "kind", "store", "recorded_root")], [1, "core", ".lineage", ROOT])
        paths = [e["path"] for e in m["files"]]
        self.assertEqual(paths, sorted(paths))
        entry = next(e for e in m["files"] if e["path"] == "results-hello/summary.txt")
        self.assertEqual((entry["size"], entry["mtime_ms"]), (9, TARGET_NS // 1_000_000))

    def test_only_data_files_kept(self):
        out = version_dir(self.base)
        self.assertEqual(sorted(p.name for p in (out / "work/cc/cc").iterdir()), ["summary.txt"])
        self.assertTrue((out / ".lineage/.history" / R1).is_file())
        self.assertTrue((out / "nextflow.config").is_file())

    def test_missing_data_file_fails(self):
        root = live_tree(self.base)
        (root / "results-hello/upper/upper.txt").unlink()
        argv = ["--kind", "core", "--version", "x", "--image", "i", "--recorded-root", ROOT,
                str(root), str(self.base / "out")]
        with self.assertRaises(SystemExit) as cm:
            export.main(argv)
        msg = str(cm.exception.code)
        self.assertIn(f"lid://{R1}/upper/upper.txt", msg)
        self.assertIn("results-hello/upper/upper.txt", msg)


class IdsTests(Tmp):
    def test_flat_and_spec_envelopes(self):
        want = {"RUN1": "lid://" + R1, "RUN2": "lid://" + R2,
                "TASK_SUMMARIZE_RUN1": "lid://" + T1, "TASK_SUMMARIZE_RUN2": "lid://" + T2}
        for envelope in ("flat", "spec"):
            with self.subTest(envelope=envelope):
                root = live_tree(self.base / envelope, envelope)
                self.assertEqual(ids.ids(root / ".lineage"), want)


    def test_agent_ids(self):
        want = {"AGENT_QA": "lid://" + T1, "AGENT_QA_STRUCTURED": "lid://" + T2,
                "AGENT_SHOUTY": "lid://" + "e" * 32, "RUN1": "lid://" + R1}
        self.assertEqual(ids.agent_ids(agent_tree(self.base) / ".lineage"), want)


class CheckTests(Tmp):
    def setUp(self):
        super().setUp()
        self.d = version_dir(self.base)

    def assertFails(self, needle):
        errs = check.check_version_dir(self.d)
        self.assertTrue(any(needle in e for e in errs), errs)

    def test_valid_dir_passes(self):
        self.assertEqual(check.check_version_dir(self.d), [])

    def test_size_mismatch_fails(self):
        (self.d / "results-hello/summary.txt").write_text("lines: 33\n")
        self.assertFails("results-hello/summary.txt: size")

    def test_sha_mismatch_fails(self):
        (self.d / "results-hello/summary.txt").write_text("lines: 4\n")
        self.assertFails("results-hello/summary.txt: sha256")

    def test_unlisted_fileoutput_fails(self):
        m = manifest(self.d)
        m["files"] = [e for e in m["files"] if e["path"] != "work/cc/cc/summary.txt"]
        write_manifest(self.d, m)
        self.assertFails("work/cc/cc/summary.txt: FileOutput not in manifest")

    def test_extra_entry_fails(self):
        (self.d / "extra.txt").write_text("x")
        m = manifest(self.d)
        m["files"].append({"path": "extra.txt", "size": 1, "mtime_ms": 0,
                           "sha256": "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881"})
        write_manifest(self.d, m)
        self.assertFails("extra.txt: manifest entry is not a FileOutput")

    def test_traversal_rejected(self):
        for bad in ("../../etc/passwd", "/etc/passwd"):
            with self.subTest(bad=bad):
                m = manifest(self.d)
                m["files"].append({"path": bad, "size": 0, "mtime_ms": 0, "sha256": ""})
                write_manifest(self.d, m)
                self.assertFails(f"unsafe path {bad!r}")

    def test_host_path_rejected(self):
        (self.d / "oracle/list.out").write_text("/home/x\n")
        self.assertFails("oracle/list.out: contains host marker '/home/'")

    def test_forbidden_name_rejected(self):
        (self.d / "oracle/list.out").write_text("someuser\n")
        errs = check.check_version_dir(self.d, forbid=["someuser"])
        self.assertTrue(any("someuser" in e for e in errs), errs)

    def test_host_path_shapes_only(self):
        (self.d / "nextflow.config").write_text('agent { runner = "langchain4j" } // runner\n')
        self.assertEqual(check.check_version_dir(self.d, forbid=["/home/runner"]), [])
        for leak in ("/home/runner/work/x", "/__w/baqueano", "/github/workspace/x", "/Users/dev/x"):
            with self.subTest(leak=leak):
                (self.d / "nextflow.config").write_text(leak + "\n")
                self.assertFails("nextflow.config: contains host marker")

    def test_missing_shape_fails(self):
        (self.d / "shape.txt").unlink()
        self.assertFails("missing shape.txt")

    def test_stale_shape_fails(self):
        (self.d / "oracle/list.out").write_text("changed\n")
        self.assertFails("shape.txt is stale")

    def test_agent_cases(self):
        d = version_dir(self.base / "agent", kind="agent")
        self.assertEqual(check.check_version_dir(d), [])
        (d / "oracle/view-agentrun-shouty.out").unlink()
        errs = check.check_version_dir(d)
        self.assertTrue(any("oracle/view-agentrun-shouty.out: missing" in e for e in errs), errs)
        shutil.rmtree(d / f".lineage/{'e' * 32}")
        errs = check.check_version_dir(d)
        self.assertTrue(any("AgentRun with tools" in e for e in errs), errs)

    def test_missing_case_file_fails(self):
        (self.d / "oracle/list.err").unlink()
        self.assertFails("oracle/list.err: missing")

    def test_empty_root_fails(self):
        empty = self.base / "empty"
        empty.mkdir()
        self.assertEqual(check.main([str(empty)]), 1)
        self.assertEqual(check.main([str(self.base / "absent")]), 1)
        root = self.base / "lineage"
        root.mkdir()
        shutil.move(str(self.d), str(root / "26.10.7"))
        self.assertEqual(check.main([str(root)]), 0)


class ShapeTests(Tmp):
    def test_volatile_tokens_masked(self):
        a = ("2026-11-26 21:25:24 UTC\ttender_booth\t387602f4-64c2-4762-aa2e-037a8336ca4c\t"
             "lid://d3aad5e9507a67cf4dc011874f1b40f5 work/3c/fe0a1b2c3d4e5f60718293a4b5c6d7 "
             "2026-11-26T21:25:23.668405139Z")
        b = ("2026-11-27 08:01:02 UTC\tzen_planck  \t228bf7e7-b09b-4cb5-ad6f-cdad28d834b9\t"
             "lid://53eff9d0e4499741b854d37e0d677ded work/9a/00112233445566778899aabbccddee "
             "2026-11-27T08:01:01.1Z")
        self.assertEqual(shape.mask(a, ["tender_booth"]), shape.mask(b, ["zen_planck"]))
        self.assertNotEqual(shape.mask("exit 1", []), shape.mask("exit 2", []))

    def test_new_field_changes_shape(self):
        d = version_dir(self.base)
        before = shape.shape(d)
        p = d / ".lineage" / R1 / ".data.json"
        r = json.loads(p.read_text())
        r["spec"]["foo"] = "x"
        p.write_text(json.dumps(r))
        after = shape.shape(d)
        self.assertNotEqual(before, after)
        self.assertIn("record WorkflowRun spec.foo str", after)

    def test_find_order_ignored(self):
        d = version_dir(self.base)
        lines = ["lid://" + R1 + "/a.txt", "lid://" + R2 + "/b.txt"]
        shapes = []
        for case in ("find-fileoutput", "find-fileoutput", "list", "list"):
            (d / f"oracle/{case}.out").write_text("\n".join(lines) + "\n")
            shapes.append(shape.shape(d))
            lines.reverse()
        self.assertEqual(shapes[0], shapes[1])
        self.assertNotEqual(shapes[2], shapes[3])
        # 25.04 prints a JSON array: only the last element lacks the trailing comma
        arrays = [f'[\n  "lid://{x}/{x[0]}.txt",\n  "lid://{y}/{y[0]}.txt"\n]\n' for x, y in ((R1, R2), (R2, R1))]
        for i, text in enumerate(arrays):
            (d / "oracle/find-fileoutput.out").write_text(text)
            shapes[i] = shape.shape(d)
        self.assertEqual(shapes[0], shapes[1])


class StubTests(unittest.TestCase):
    def setUp(self):
        self.srv = llm_stub.server(0)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.srv.server_address[1]}"

    def tearDown(self):
        self.srv.shutdown()
        self.srv.server_close()

    def post(self, path, body):
        req = urllib.request.Request(self.url + path, json.dumps(body).encode(),
                                     {"content-type": "application/json"})
        with urllib.request.urlopen(req, timeout=5) as r:
            return json.load(r)

    def test_chat_completion_json_schema(self):
        schema = {"type": "object", "required": ["answer", "confidence", "tags"],
                  "properties": {"answer": {"type": "string"}, "confidence": {"type": "number"},
                                 "tags": {"type": "array", "items": {"type": "string"}}}}
        body = {"model": "m", "messages": [], "stream": False,
                "response_format": {"type": "json_schema", "json_schema": {"name": "Output", "schema": schema}}}
        content = json.loads(self.post("/v1/chat/completions", body)["choices"][0]["message"]["content"])
        self.assertIsInstance(content["answer"], str)
        self.assertIsInstance(content["confidence"], float)
        self.assertTrue(all(isinstance(t, str) for t in content["tags"]))
        plain = self.post("/v1/chat/completions", {"model": "m", "messages": []})
        self.assertEqual(plain["choices"][0]["message"]["content"], llm_stub.TEXT)

    def test_unknown_path_501(self):
        with self.assertRaises(urllib.error.HTTPError) as cm:
            self.post("/v1/responses", {})
        self.assertEqual(cm.exception.code, 501)
        self.assertIn("/v1/responses", json.load(cm.exception)["error"]["message"])


class RestoreTests(Tmp):
    def test_restores_ms_and_checks_size(self):
        src = version_dir(self.base)
        cp = self.base / "copy"
        shutil.copytree(src, cp, copy_function=shutil.copy)  # like git: mtimes lost
        restore_mtimes.restore(cp)
        for e in manifest(cp)["files"]:
            self.assertEqual((cp / e["path"]).stat().st_mtime_ns // 1_000_000, e["mtime_ms"], e["path"])
        (cp / "results-hello/summary.txt").write_text("x")
        with self.assertRaisesRegex(ValueError, "results-hello/summary.txt"):
            restore_mtimes.restore(cp)


if __name__ == "__main__":
    unittest.main()
