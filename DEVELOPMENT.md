# Development Guide

## Parameter Generation

To perform parameter generation, run the following command:
```$xslt
./scripts/parameter_generation.sh
```
This will create new snarkOS parameters, a new genesis block, and new test data. The results are stored in their
respective folders in `snarkos-parameters` and `snarkos-testing`.

### When to Regenerate Parameters

If your changes include modifications to DPC circuits, the block architecture, or the transaction architecture, 
the snarkOS public parameters will need to be regenerated.
