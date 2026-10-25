"""Coordinator self-tests; fixtures are deliberately not general task solutions."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

from grade import build, check_cases, run, submit
from preloaded import build_preloaded
from protocol import BASE, MASK, PROJECT, PUBLIC, TASKS, check_output, digest, expected, pool_checksum, pool_text


class HarnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="rivet-pilot-test-")
        cls.root = Path(cls.temp.name)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_oracles_and_output_contract(self):
        answers = {"two_sum": [[0, 1], [1, 2], [0, 1]],
                   "add_two_numbers": [[7, 0, 8], [0], [8, 9, 9, 9, 0, 0, 0, 1]],
                   "median": [2.0, 2.5]}
        for task, cases in PUBLIC.items():
            for case, answer in zip(cases, answers[task]):
                self.assertEqual(expected(task, case), answer)
                self.assertTrue(check_output(task, case, json.dumps(answer))["ok"])
        for malformed in ("[0,0]", "[true,1]", "[1.0,0]", "[0,1] garbage", "null"):
            self.assertFalse(check_output("two_sum", PUBLIC["two_sum"][0], malformed)["ok"])
        self.assertTrue(check_output("two_sum", PUBLIC["two_sum"][0], "[1,0]")["ok"])
        self.assertFalse(check_output("median", PUBLIC["median"][0], "NaN")["ok"])

    def test_submission_snapshots_limits_and_feedback(self):
        directory = self.root / "submissions"
        workspace = directory / "worker"
        workspace.mkdir(parents=True)
        case = {"a": [2, 7], "target": 9}
        corpus = directory / "corpus.json"
        corpus.write_text(json.dumps({"development": {"two_sum": [case]}}))
        config = {"trials": [{"workspace": str(workspace), "id": "fixture", "language": "c", "task": "two_sum"}],
                  "results": str(directory / "results"), "builds": str(directory / "builds"),
                  "rivet": str(PROJECT / "target/release/rivet"), "corpus": str(corpus)}
        config_path = directory / "config.json"
        config_path.write_text(json.dumps(config))
        source = workspace / "solution.c"
        values = ["not valid C", '#include "api.h"\nPair solve(const int64_t*a,size_t n,int64_t t){(void)a;(void)n;(void)t;return (Pair){0,0};}',
                  '#include "api.h"\nPair solve(const int64_t*a,size_t n,int64_t t){(void)a;(void)n;(void)t;return (Pair){0,1};}']
        for index, text in enumerate(values, 1):
            text += "\n"
            source.write_text(text)
            with contextlib.redirect_stdout(io.StringIO()) as feedback:
                result = submit(workspace, config_path)
            self.assertEqual(result["attempt"], index)
            self.assertEqual(result["passed"], index == 3)
            snapshot = directory / f"results/fixture/attempt-{index}/solution.c"
            self.assertEqual(snapshot.read_text(), text)
            self.assertEqual(result["files"]["solution.c"], digest(snapshot))
            if index == 1:
                self.assertIn("compiler_diagnostics", json.loads(feedback.getvalue()))
            if index == 2:
                self.assertIn("first_failed_case", json.loads(feedback.getvalue()))
        with self.assertRaises(ValueError):
            submit(workspace, config_path)
        self.assertFalse((directory / "results/fixture/submitting.lock").exists())

    def test_c_rust_capture_checksum_and_strict_parser(self):
        fixtures = {
            "two_sum": ('#include "api.h"\nPair solve(const int64_t*a,size_t n,int64_t t){(void)a;(void)n;(void)t;return (Pair){1,0};}',
                        'pub fn solve(_: &[i64], _: i64)->(usize,usize){(1,0)}', {"a": [2, 7], "target": 9}),
            "add_two_numbers": ('#include "api.h"\n#include <stdlib.h>\nNode*solve(const Node*a,const Node*b){(void)a;(void)b;Node*r=malloc(sizeof(*r));if(!r)abort();r->value=0;r->next=NULL;return r;}',
                                'pub fn solve(_: &Option<Box<Node>>, _: &Option<Box<Node>>)->Option<Box<Node>>{Some(Box::new(Node{value:0,next:None}))}',
                                {"a": [0], "b": [0]}),
            "median": ('#include "api.h"\ndouble solve(const int64_t*a,size_t n,const int64_t*b,size_t m){(void)a;(void)n;(void)b;(void)m;return -0.5;}',
                       'pub fn solve(_: &[i64], _: &[i64])->f64{-0.5}', {"a": [-1], "b": [0]}),
        }
        for task, (c, rust, case) in fixtures.items():
            directory = self.root / task
            directory.mkdir()
            cases = [case] * 40
            pool = directory / "pool.txt"
            pool.write_text(pool_text(task, cases))
            for language, source_text, ext in (("c", c, "c"), ("rust", rust, "rs")):
                source = directory / f"fixture.{ext}"
                source.write_text(source_text + "\n")
                built = build(language, task, source, directory / language)
                self.assertTrue(built["ok"], built)
                self.assertTrue(check_cases(task, built["executable"], [case])["ok"])
                for mode in ("evaluate", "control"):
                    result = run([built["executable"], "--bench", pool, "3", "2", mode])
                    self.assertEqual(result["exit_code"], 0, result)
                    payload = json.loads(result["stdout"])
                    self.assertEqual(payload["checksum"], (pool_checksum(task, cases) * 3) & MASK)
                    self.assertEqual(payload["calls"], 120)
                    self.assertEqual(payload["warmup_calls"], 80)
                result = run([built["executable"]], stdin=b"+1 0 1 0\n")
                self.assertNotEqual(result["exit_code"], 0)

    def test_rivet_boundaries_against_trusted_sources(self):
        corpus = json.loads((BASE / "corpora.json").read_text())
        names = {"two_sum": "two-sum", "add_two_numbers": "add-two-numbers", "median": "median-two-sorted-arrays"}
        for task in TASKS:
            built = build("rivet", task, PROJECT / f"examples/{names[task]}.rvb", self.root / f"rivet-{task}")
            self.assertTrue(built["ok"], built)
            preloaded = build_preloaded(task, built["executable"], self.root / f"rivet-{task}/preloaded")
            self.assertTrue(check_cases(task, built["executable"], corpus["development"][task])["ok"])
            for family, cases in corpus["performance"][task].items():
                pool = self.root / f"{task}-{family}.txt"
                pool.write_text(pool_text(task, cases))
                for mode in ("evaluate", "control"):
                    result = run([preloaded["executable"], "--bench", pool, "3", "1", mode])
                    self.assertEqual(result["exit_code"], 0, result)
                    payload = json.loads(result["stdout"])
                    self.assertEqual(payload["checksum"], (pool_checksum(task, cases) * 3) & MASK)
                    self.assertTrue(all(t > 0 for t in payload["ticks_per_case"]))


if __name__ == "__main__":
    unittest.main()
