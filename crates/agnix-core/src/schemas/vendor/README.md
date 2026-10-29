# Bundled vendor constraints

These six JSON files contain the functional constraints of the published Pi v1.4.0 and Antigravity CLI schemas. Prose annotations were removed. Property names and validation keywords are preserved.

`sources.json` records each primary URL, original response SHA-256, normalized constraint SHA-256 and verification date. No user-supplied schema is fetched or compiled. The `jsonschema` dependency has network and file resolution features disabled.

Pi's source is published in [earendil-works/pi at v1.4.0](https://github.com/earendil-works/pi/tree/v1.4.0); its MIT notice is included in `PI-LICENSE`. Antigravity's settings and plugin constraints are published by Google at the URLs in `sources.json`. No license for those published schemas is asserted here.

The adjacent `grok_contract.json` is an agnix-authored translation of public parser field contracts and current documentation. Its provenance is [NATIVE-TOOL-CONTRACTS.md](../../../../../knowledge-base/NATIVE-TOOL-CONTRACTS.md); it is not an upstream JSON Schema.
