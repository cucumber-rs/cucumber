# capture-runner

An internal-only test-helper CLI application which
executes a single cucumber scenario using the
cucumber-rust framework.

The steps of the scenario will attempt to print
to stdout and stderr.

The cucumber runner can be minimally configured
using command line arguments.

## Example usage

To run the cucumber test with enable_capture on:
```shell script
capture-runner true
```

To run the cucumber test with enable_capture off:
```shell script
capture-runner false
```
