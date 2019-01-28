# Queueing simulator

This repository contains a simulator for queueing network designed for quick experimentations. It supports both FCFS and PS strategies, stateful transitions between queues, etc.

## Current experiments
 + **[Fog admission control](#fog-admission-control)**
 + **[Autoscaling](#autoscaling)**

## Fog admission control

A queueing network to simulate a Fog-acceptance module to control application latency. See [1] for further description of the scenario.

To run it, set the parameters in `src/fog_cloud_sim`, then run `cargo run fog [mode]` where `[mode]`is one of:
 + `blind`for a blind admission-control module
 + `lfu`for the oracle-based admission-control
 + `lru`for the LRU-AC implemented with an actual LRU
 + `abf`for the LRU-AC implemented with an ABF cache

To derive the parameters of the Admission-Control module, a Jupyter notebook is available in `helpers/Fog admission control optimization`. You can use it by installing [Jupyter](https://jupyter.org/).
Note that this notebook must be ran with a [Python 2 kernel](https://github.com/jupyter/jupyter/issues/71)

[1] : Enguehard, Marcel, Giovanna Carofiglio and Dario Rossi. “[A Popularity-Based Approach for Effective Cloud Offload in Fog Deployments](https://enguehard.org/papers/fog-cloud-itc30.pdf).” _2018 30th International Teletraffic Congress (ITC 30)_

## Autoscaling

To be completed soon...
