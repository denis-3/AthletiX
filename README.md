# AthletiX
AthletiX is a social network which allows fans to actively participate in their athletes' exclusive blockchain communities. Built for the OPLxSei hackathon (Sep. 2023).


## Contract - Local Development
Make sure to `cd` into `/contract`.

There are three commands that come pre-packaged in this repo:

`wasm-build`: Build the smart contract.
`wasm-test`: Run tests on the smart contract.
`wasm-build-debug`: Build the smart contract (debug mode).

Run `cargo command-name` to execute any one of the commands and replace `command-name` with a command listed above.

## Contract - Testnet and Mainnet Deployment
This contract has quite a large size when compiled (about a few MB). Therefore it is recommended to optimize the deployment size using the [Rust optimizer](https://github.com/CosmWasm/rust-optimizer). This tool can get the size down of the compiled `.wasm` binary to just about 200KB!

The contract currently deployed to the Sei testnet at: https://www.seiscan.app/atlantic-2/contracts/sei1hc9xmnc6lmckxu29tk35arc02cfxwk24nphjruh2x9m8f2ap3kese7jeh8
