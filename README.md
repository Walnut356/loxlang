# Benchmarks

## Clox (baseline)

compiled via MSVC

| bench            | time (base) | time (NAN Boxing) |
| ---------------- | ----------- | ----------------- |
| binary_trees     | 3.761s      | 2.769s            |
| equality         | 2.696s      | 1.318s            |
| fib              | 0.78s       | 0.7s              |
| instantiation    | 0.726s      | 0.701s            |
| invocation       | 0.235s      | 0.205s            |
| method_call      | 0.152s      | 0.131s            |
| properties       | 0.351s      | 0.323s            |
| string_equality* | 0.656s      | 0.6s              |
| trees            | 7.126s      | 5.036s            |
| zoo_batch        | 5550        | 6214              |
| zoo              | 0.278s      | 0.249s            |

* Note: I did have to add Value deduplication to `chunck.c`'s `addConstant` function to get this to run

## rlox

| bench            | time (base) |
| ---------------- | ----------- |
| binary_trees     | 4.123s      |
| equality         | 5.345s      |
| fib              | 2.845s      |
| instantiation    | 1.411s      |
| invocation       | 0.926s      |
| method_call      | 0.551s      |
| properties       | 1.313s      |
| string_equality* | 2.325s      |
| trees            | 13.725s     |
| zoo_batch        | 1622        |
| zoo              | 1.040s      |