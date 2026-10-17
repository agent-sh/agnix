---
name: steps-and-samples
displayName: Steps and Samples
description: Power fixture with step headings, code samples and a repeated keyword
keywords:
  - secrets
  - vault
  - Secrets
---
# Steps and Samples

## Overview

## Step 1: Store the credential

Read the value from the vault instead of hardcoding it:

```rust
let password: SecretString = SecretString::new(value.into());
```

## Step 2: Rotate it

## Troubleshooting

Check the vault audit log.
