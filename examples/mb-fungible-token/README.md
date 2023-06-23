Mintable Burnable Fungible Token (FT)
===================

Example implementation of a extension of [Fungible Token] contract which uses [near-contract-standards] and extends by minting and burning.

  [Fungible Token]: https://nomicon.io/Standards/Tokens/FungibleTokenCore.html
  [near-contract-standards]: https://github.com/near/near-sdk-rs/tree/master/near-contract-standards

NOTES:
 - The maximum balance value is limited by U128 (2**128 - 1).
 - JSON calls should pass U128 as a base-10 string. E.g. "100".
 - This does not include escrow functionality, as `ft_transfer_call` provides a superior approach. An escrow system can, of course, be added as a separate contract.

## Installation
To build rust dependencies:
```bash
cargo build
```

## Building
To build run:
```bash
./build.sh
```

## Post building
Wasm file must be cleaned with wasm-opt
```bash
wasm-opt -Oz --signext-lowering res/mb_fungible_token.wasm -o fungible_fixed.wasm
```

## Changelog

### `1.0.0`

- Switched form using [NEP-21](https://github.com/near/NEPs/pull/21) to [NEP-141](https://github.com/near/NEPs/issues/141).

### `0.3.0`

#### Breaking storage change

- Switching `UnorderedMap` to `LookupMap`. It makes it cheaper and faster due to decreased storage access.

