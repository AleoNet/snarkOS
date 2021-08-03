# Serialization

## Source Structure

The authoritative source of what data we need to represent in our `SerialX` structures as decribed below are excluseively a superset of all definitions in snarkVM.

## Motivations

We want to decouple snarkOS from specific network implementations in snarkVM alongside implementation-specific details. This is particularly driven by storage standardization and managing Rust's type system in a multi-network environment. We want to avoid leaking the complex types defined in snarkVM to the entirety of snarkOS.

## Current State

Currently, all serialized formats are defined exclusively by snarkVM to maintain backward compatibility until testnet2 is launched or a new format & migration script is implemented.

## Stages of Representation

All of the following types:

`SerialBlock`, `SerialTransaction`, `SerialBlockHeader`, `SerialRecord`

are subject to the following serialization/deserialization stages

* `snarkvm_dpc::*::Block<?>` -> `SerialBlock` -> raw bytes
* `snarkvm_dpc::*::Transaction<?>` -> `SerialTransaction` -> raw bytes
* `snarkvm_dpc::*::BlockHeader` -> `SerialBlockHeader` -> raw bytes
* `snarkvm_dpc::*::Record<?>` -> `SerialRecord` -> raw bytes

In practice, until a new serialization format is implementated, there is some conversion back and forth as needed.

