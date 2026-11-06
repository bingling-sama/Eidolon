# Changelog

All notable changes to Eidolon will be documented in this file.
## [0.0.5-alpha.3] - 2026-11-06

### CI/CD

- *(release)* Bump git-cliff-action to v4 ([1edd512](https://github.com/bingling-sama/eidolon/commit/1edd512afc8ab1d357b057748ecbb73fcdcf8404))

## [0.0.5-alpha.2] - 2026-11-06

### CI/CD

- *(release)* Install cross from git main for aarch64 build ([c8ca41c](https://github.com/bingling-sama/eidolon/commit/c8ca41cc38e19eb9b7a6ace42711430cef02a37b))

## [0.0.5-alpha.1] - 2026-11-06

### CI/CD

- Speed up workflows with caching and drop x86_64-apple-darwin ([7280e1f](https://github.com/bingling-sama/eidolon/commit/7280e1f68731ea736d0217139258cea5f476fe07))

## [0.0.5-alpha] - 2026-11-06

### Bug Fixes

- *(server)* Simplify back view custom yaw and document server in workspace layout ([c006adb](https://github.com/bingling-sama/eidolon/commit/c006adb68cba1820c04378b0003bf950bb17e2a8))
- *(server)* Support concurrent character fetch and nametag_see_through field ([9673f5b](https://github.com/bingling-sama/eidolon/commit/9673f5bb71c1a6fdb3b6722a3e66f4d7b2f1ff75))
- *(server)* Resolve clippy warnings on enum variant size and type complexity ([ceb7ad1](https://github.com/bingling-sama/eidolon/commit/ceb7ad11cf66ebefeabe73f1c71f4c0e0494739e))

### CI/CD

- Add separate release workflow for server to GHCR ([55d489f](https://github.com/bingling-sama/eidolon/commit/55d489fbfca77100e2af471857302cd1d7c67409))
- Trigger server docker release automatically on main changes using commit hash ([2c54185](https://github.com/bingling-sama/eidolon/commit/2c54185889956e21577056c907c7c6dd9e508b0e))

### Features

- *(server)* Align dimensions, fakeAnimation posture, camera, and #FFFFFF background with SkinRenderMC ([5e34c17](https://github.com/bingling-sama/eidolon/commit/5e34c173e2cc06822b91b846a836b8f4056c2478))
- *(server)* Update default definition to 2.0 ([882bd9e](https://github.com/bingling-sama/eidolon/commit/882bd9e03b016c4ab9c5ddd7d54cf28062aebf3f))
- *(server)* Add multi-character scene render endpoint ([a1bca6f](https://github.com/bingling-sama/eidolon/commit/a1bca6f2af037867b2c506769ce5519f20d991ea))

### Refactor

- *(server)* Move Dockerfile and docker-compose.yml into examples/server ([29746ba](https://github.com/bingling-sama/eidolon/commit/29746baaff8aafc2cbeaa1372c0b3afed5d8717f))

## [0.0.4-alpha] - 2026-11-03

### Bug Fixes

- Address code review findings for multi-character rendering ([b49c6e1](https://github.com/bingling-sama/eidolon/commit/b49c6e10de197fe7108082053ffdd329987f5191))
- Address review findings for docs, nametag depth, wasm elytra, and config errors ([403e490](https://github.com/bingling-sama/eidolon/commit/403e490c6393159f22db65aa2e7df48d21da0a1b))
- Align auto-framing target centering, sync library docs, and add scene config docs ([f690e8a](https://github.com/bingling-sama/eidolon/commit/f690e8a1f886518b34308ccd0bd7ec45dd07e7d9))
- *(server)* Address review findings for SkinRenderMC compatibility ([7d3d259](https://github.com/bingling-sama/eidolon/commit/7d3d2597ce0b2ff920787c26104849e6950bef01))
- *(server)* Encode query usernames, fix json_all cache headers, and add http tests ([369c334](https://github.com/bingling-sama/eidolon/commit/369c334c47aabc9e46fd09282d6b729fd0954182))
- *(server)* Handle upstream rate limits, fallback missing skin, and fix test worker lifecycle ([bf3ad4f](https://github.com/bingling-sama/eidolon/commit/bf3ad4f37f35f5c9d41840910ce328a779537dfe))
- *(server)* Merge POST query parameters, enforce PNG, and update docs ([9c3b331](https://github.com/bingling-sama/eidolon/commit/9c3b3317e79291a7ddf0f67f848363b3413910c3))
- *(server)* Improve form error handling and unify both view opaque blending ([2dc4173](https://github.com/bingling-sama/eidolon/commit/2dc4173de918abf92f933e2ac5d284d848c9fe4e))
- *(server)* Map Mojang session API 400 status to BadRequest ([f589779](https://github.com/bingling-sama/eidolon/commit/f5897794b31f14a58732379441c9111b1f03d4f5))
- *(server)* Map LittleSkin session API 400 status to BadRequest ([a1c3c23](https://github.com/bingling-sama/eidolon/commit/a1c3c230def8134647d0c8c644071655d62d332a))

### CI/CD

- Support crates.io OIDC Trusted Publishing with fallback token ([c9282e8](https://github.com/bingling-sama/eidolon/commit/c9282e8a1b54f6a4e52c8a1ce8e6c2aa98fe5d31))

### Documentation

- Add package READMEs for eidolon-core, eidolon-cli, and @eidolonmc/wasm ([4c8bc38](https://github.com/bingling-sama/eidolon/commit/4c8bc3856455183a55936a5f08f64bd3daf0ebe7))
- Add multi-character scene preview image ([a57d51f](https://github.com/bingling-sama/eidolon/commit/a57d51fd2d1974650fd13ea3ac900fc9760b92b2))

### Features

- *(core)* Introduce Scene data structure and camera framing helpers ([cadb231](https://github.com/bingling-sama/eidolon/commit/cadb231334a38716c8aabc81b4305196ee682748))
- *(renderer)* Support multi-character rendering in single render pass ([265fc5c](https://github.com/bingling-sama/eidolon/commit/265fc5cfa9dcce79ba8d3a2f90835b217df5f8ef))
- *(cli)* Support multi-character scene configuration and rendering ([d484f84](https://github.com/bingling-sama/eidolon/commit/d484f8495402a4618b784ab0a362b008f793f8fc))
- *(wasm)* Add multi-character entity manipulation API to CanvasRenderer ([4caca82](https://github.com/bingling-sama/eidolon/commit/4caca82b433344c5c7666d3775b1b51266d86f4a))
- *(server)* Add full API compatibility with SkinRenderMC ([0ebe08d](https://github.com/bingling-sama/eidolon/commit/0ebe08d7929d09e0d0b0253d6260c4a82269571c))
- *(server)* Add visual preview image, improve POST parameter parsing, and enhance missing skin fallback ([d2db7c4](https://github.com/bingling-sama/eidolon/commit/d2db7c4221fe7bc343374779c697ba4a55b182a0))

### Refactor

- *(core)* Narrow chunk constants visibility and add buffer scaling unit tests ([3e791a3](https://github.com/bingling-sama/eidolon/commit/3e791a35f9da68537c807c2be7f6b56c0a4facd5))
- *(server)* Relocate preview image, refine JSON sniffing, and handle 400 status ([914ad07](https://github.com/bingling-sama/eidolon/commit/914ad075d207322f6670bb16717b50d9c14c9f4f))
- Scope PR changes strictly to server and restore docs structure ([7720f20](https://github.com/bingling-sama/eidolon/commit/7720f20b91d0382a2491b918d3ee5d90cb91fcab))

### Styling

- Apply cargo fmt across workspace ([c0370d0](https://github.com/bingling-sama/eidolon/commit/c0370d0ef8a46aa2af73a476cee5079d8e42508d))

## [0.0.3-alpha] - 2026-11-02

### Bug Fixes

- Address security issues found during audit ([4c1d5a6](https://github.com/bingling-sama/eidolon/commit/4c1d5a696bfb2abf8bd5a53d263b5d7e189b9c9d))
- *(core)* Resolve clippy and compiler warnings across workspace ([3b9f5f8](https://github.com/bingling-sama/eidolon/commit/3b9f5f84a4e1335ec73a0b04ac715d4225e6df22))
- *(ci)* Sync package-lock.json with vite 8 and playwright upgrade ([9a821dc](https://github.com/bingling-sama/eidolon/commit/9a821dc957c93d13a1ca90170fbfaf1a9b4f1dd6))
- *(ci)* Align windows dependencies, update Docker rust base, and support runner snapshot hash ([a38d498](https://github.com/bingling-sama/eidolon/commit/a38d498a509b562e623a1155ffbb9fd185b21cdf))
- *(model)* Remove the baked per-part rotations from classic.obj ([528ad0e](https://github.com/bingling-sama/eidolon/commit/528ad0ecb3066516cae16673ee619b91afe69cb8))
- *(camera)* Zoom with the lens instead of moving the camera ([06cc196](https://github.com/bingling-sama/eidolon/commit/06cc196c323756038605f48b7900a53195aba028))
- *(animation)* Mirror the arm roll between both arms ([1a41211](https://github.com/bingling-sama/eidolon/commit/1a41211a8238650e2adeb12e16287575cde026ee))
- *(cli)* Keep the preview window attributes immutable off Windows ([de78e00](https://github.com/bingling-sama/eidolon/commit/de78e004c66647c78a095933079be3a9e3b276e4))
- *(cli)* Tighten the composition preview path ([adbe731](https://github.com/bingling-sama/eidolon/commit/adbe7312094f6b5c0dd1ba0180f924754b0364ad))
- *(ci)* Fix crates packaging, npm trusted publishing and macos runner ([5b17efe](https://github.com/bingling-sama/eidolon/commit/5b17efee7a7b8a55011302bcb26b387b42d61a83))

### CI/CD

- Fix wasm build step and futures-channel dependency in deploy workflow ([9e97552](https://github.com/bingling-sama/eidolon/commit/9e975521574477097faad3061bf85ef8e5281950))
- Add CI workflows for Rust, WASM E2E, Docker, and Releases ([68c6c2d](https://github.com/bingling-sama/eidolon/commit/68c6c2d8da213125f27cd08d2cd9335fb038db9c))
- Configure automated release workflow and git-cliff tooling ([ed86cb4](https://github.com/bingling-sama/eidolon/commit/ed86cb4153431d41249c1694c41af1add1ccba04))

### Documentation

- Enhance documentation and improve code clarity ([f3b01bf](https://github.com/bingling-sama/eidolon/commit/f3b01bf0855a7d34e6d1358608b5418f716b0d5c))
- Update README and resources documentation ([182e2de](https://github.com/bingling-sama/eidolon/commit/182e2ded90ef88d061016a5cc8da50cf7da75444))
- Fix typo and align prerequisites across docs ([3ebf99e](https://github.com/bingling-sama/eidolon/commit/3ebf99e9a2c2bcd6805c898c90f44cd8d50e9107))
- Update documentation for clarity and new features ([9afd515](https://github.com/bingling-sama/eidolon/commit/9afd5150a4c5870c88c78a76e82a5e5ae5f09d7d))
- Update documentation for monorepo and modular crate structure ([42fd405](https://github.com/bingling-sama/eidolon/commit/42fd405608c4a8322689a536a4d9658e4d3d394e))
- Add browser compatibility matrix and WebGL2 fallback notes ([b9604ec](https://github.com/bingling-sama/eidolon/commit/b9604ec1aa56988bd318b0a4a18fff90adf6dc1b))
- *(development)* Correct the core unit test count ([86435ee](https://github.com/bingling-sama/eidolon/commit/86435ee57375ee53829cb649ac776914f2cff146))

### Features

- Add new postures and enhance CLI posture handling ([a4ef002](https://github.com/bingling-sama/eidolon/commit/a4ef002622206fd761ab60021a718ccc3c3623a8))
- Add static cape rendering ([9966e5b](https://github.com/bingling-sama/eidolon/commit/9966e5b03ef7cc6b95af3b7754aff0ab9c85d736))
- Add interactive preview camera controls ([b19bb99](https://github.com/bingling-sama/eidolon/commit/b19bb99ab2b8cddaa0c4a8edce146e2cc30e9b61))
- *(wasm)* Add nametag support to web bindings and example ([24a517b](https://github.com/bingling-sama/eidolon/commit/24a517b2174df2c9b66d5c5f9ee2fcbebca685f4))
- *(render)* Switch to pure ambient lighting ([75773d2](https://github.com/bingling-sama/eidolon/commit/75773d2922b67e1772b0645ec4bd281c3497fb67))
- *(renderer)* Switch nametag to minecraft.ttf with 16x16 unifont fallback ([b893744](https://github.com/bingling-sama/eidolon/commit/b893744fe0938844c25fcbe8c041d03a00fb8a67))
- *(wasm)* Redesign studio layout and unify accessory workflow ([1772053](https://github.com/bingling-sama/eidolon/commit/17720538ec7fad0c743d1eac8bbb95cfda6183e8))
- *(classifier)* Add texture classifier across core API, CLI and WASM ([0e1b313](https://github.com/bingling-sama/eidolon/commit/0e1b3133c120b6b4a6841331548ebdf51a8f2091))
- *(wasm)* Add export format selection support ([aa8f96b](https://github.com/bingling-sama/eidolon/commit/aa8f96b78cc27e55206a53270aeed0aecbe0aafe))
- *(output)* Add APNG and AVIF animated export formats ([b90d915](https://github.com/bingling-sama/eidolon/commit/b90d91517a30b2323db624f7efb669a6ef2ae3ed))
- *(examples)* Add resident HTTP rendering server ([cdc45b1](https://github.com/bingling-sama/eidolon/commit/cdc45b132cac99e174eb41b4b244077a4060b0e2))
- *(server)* Implement SkinRenderMC /url/image/both endpoint ([854e93a](https://github.com/bingling-sama/eidolon/commit/854e93a5255725bf0cb31aa19bee1109395e436b))
- Upgrade wgpu to 30 and vite to 8 ([78ecc8c](https://github.com/bingling-sama/eidolon/commit/78ecc8cd15d03e416ec0d9f052bf8d670145147d))
- *(server)* Add Dockerfile and compose setup ([44b40f4](https://github.com/bingling-sama/eidolon/commit/44b40f48cb574c4c23ef9f63f73e0ee2fe9ac4e9))
- *(wasm)* Add WebGL2 fallback support alongside WebGPU ([6bf7b4e](https://github.com/bingling-sama/eidolon/commit/6bf7b4eb1997ab9a519cedbfaba3f0350b9ae9d9))
- *(wasm)* Add webgl2 dual-backend benchmarks and fix buffer readback ([b343772](https://github.com/bingling-sama/eidolon/commit/b343772d0261dcab70ac21638acb0e1f5bf68bd3))
- *(cli)* Add a transparent preview background ([4905ac3](https://github.com/bingling-sama/eidolon/commit/4905ac3aa0c048b47294bf8a30e9df6f6e843c8a))

### Miscellaneous Tasks

- Ignore macOS metadata and dotfiles ([de9dc75](https://github.com/bingling-sama/eidolon/commit/de9dc7539684155a9d4b47bbbdac7044b2ec4bfc))
- Update open source license to MIT ([4157da2](https://github.com/bingling-sama/eidolon/commit/4157da20b6c2afc6ce167b80554a81eb56494c3f))
- *(wasm)* Add rust-analyzer configuration for wasm32 target ([b4599bd](https://github.com/bingling-sama/eidolon/commit/b4599bd9b2059eae02319446ed9ff3e821acf692))

### Performance

- *(wasm)* Optimize image encoding and fix wasm-pack output path ([6335202](https://github.com/bingling-sama/eidolon/commit/633520258c0a51fa08430081760275aa28a89cae))
- *(core)* Optimize GPU readback image buffer copy with fast-path ([72df948](https://github.com/bingling-sama/eidolon/commit/72df948e047a1a5a12467a96936054d436a7ed6e))
- *(core)* Pool offscreen render targets and staging buffers ([f621802](https://github.com/bingling-sama/eidolon/commit/f6218020117ea6e7065a426d8e01d511bbe41982))
- *(core)* Expand and isolate rendering and asset benchmark suites ([b6a019c](https://github.com/bingling-sama/eidolon/commit/b6a019ceb21e11695e259be97b56c7354c0ed317))

### Refactor

- Update default posture values and integrate into CLI arguments ([80ec76c](https://github.com/bingling-sama/eidolon/commit/80ec76cb50d8d978107c3fbf720a33fa990e4c8d))
- *(core)* Decouple asset loading and enable Send+Sync ([302e73e](https://github.com/bingling-sama/eidolon/commit/302e73ebb334ccee3d7bae821b742b27c55be9fb))
- *(assets)* Collapse skin conversion into RawSkin ([5414331](https://github.com/bingling-sama/eidolon/commit/5414331ee53db0270543d2d926af3bfa6008b38a))
- *(model)* Consolidate equipment and body mesh generation ([3713f56](https://github.com/bingling-sama/eidolon/commit/3713f56e166cce953a818660c8e1275d500a5ed2))
- *(core)* Extract rendering engine into crates/core ([42427e8](https://github.com/bingling-sama/eidolon/commit/42427e8804cedfe3c02964758e40788f75e04267))
- *(cli)* Extract CLI application into crates/cli ([02f8c95](https://github.com/bingling-sama/eidolon/commit/02f8c959cf7ede27449531d7a01045e6cf5c48b0))
- *(wasm)* Extract WASM bindings and npm package into crates/wasm ([80b75e4](https://github.com/bingling-sama/eidolon/commit/80b75e447725e833759def1185c27e69632f8b20))
- *(workspace)* Configure monorepo tooling, examples and architecture docs ([c51bc87](https://github.com/bingling-sama/eidolon/commit/c51bc87f398783f93da394d439a0205c9717b0ba))
- *(cli)* Break down main.rs into modular subcomponents ([6e66e5e](https://github.com/bingling-sama/eidolon/commit/6e66e5eac305475a8b57bbd3366ac7453b625d22))
- *(wasm)* Break down wasm lib.rs into modular subcomponents ([7574ddd](https://github.com/bingling-sama/eidolon/commit/7574ddd58ccf01d7c1784a96bc02637b642dd3d2))
- *(core)* Support thread-safe concurrent rendering ([45f64e2](https://github.com/bingling-sama/eidolon/commit/45f64e243c9976420369d3ac4fb3ce9485575d4f))

### Security

- Remove .env from tracking, add to .gitignore, create .env.example ([4f9a8b2](https://github.com/bingling-sama/eidolon/commit/4f9a8b22f7d9477df35e121721b6c383822d2268))

### Testing

- *(core)* Resolve resources/ from the manifest directory ([0e36ff2](https://github.com/bingling-sama/eidolon/commit/0e36ff2de5ab2c3ae56ae6012df21ff1d80d2a62))
- *(core)* Reuse the shared resource helper ([62c5163](https://github.com/bingling-sama/eidolon/commit/62c516311cac048a36d1dfa5475df0336acf85cc))

### Build

- Centralize workspace dependencies and update packages ([77c3f6b](https://github.com/bingling-sama/eidolon/commit/77c3f6b43f25c3501a514227c19cac0c82d5f037))

### Security

- Replace panicking unwrap()/expect() with proper error propagation ([7b1701c](https://github.com/bingling-sama/eidolon/commit/7b1701cb2213042f68a7cbdd7ba8e9a1be92f79c))

## [0.0.2-alpha] - 2026-03-24

### Refactor

- Rewrite with wgpu, replacing opengl ([23dde28](https://github.com/bingling-sama/eidolon/commit/23dde28d102e11e63291e6130cf10931ff666b6e))
- Refactor renderer module into parts, enhance code readability ([c2df82a](https://github.com/bingling-sama/eidolon/commit/c2df82ab50f6d850e20208ae4d2259316fec45f8))
- Optimize depth texture management in Renderer ([7855c91](https://github.com/bingling-sama/eidolon/commit/7855c9197c47c1cff76b6b04ff2486fe3b7edcca))

## [0.0.1-alpha] - 2025-08-24

### Docs

- Add acknowledgements to README ([d35e7bd](https://github.com/bingling-sama/eidolon/commit/d35e7bd62364fd06fce9ed165c9305e1c3e7493c))

### Features

- Restruct renderer code ([e5e2d54](https://github.com/bingling-sama/eidolon/commit/e5e2d54a0ffde3363371df8a2917b01374ddfe08))
- Implement orbital camera controls ([b52e9ca](https://github.com/bingling-sama/eidolon/commit/b52e9ca21eece7a67b215a26c13970c8eb0ba886))
- Refactor model for animation and add walking cycle ([3698c39](https://github.com/bingling-sama/eidolon/commit/3698c39039f19155edd1b24a4456313057c98757))
- Add benchmarking with criterion ([fc40b78](https://github.com/bingling-sama/eidolon/commit/fc40b78514f0ae837784eec036b82572b9d6beed))

### Fix

- Resolve Z-fighting on character model ([4e03ce7](https://github.com/bingling-sama/eidolon/commit/4e03ce7904bde2614d1fcf2b710994988661c9c9))

### Refactor

- Decompose renderer into smaller components ([3456f0c](https://github.com/bingling-sama/eidolon/commit/3456f0ca14730226387923d2d7bf90b75c956cc9))
- Rename project to eidolon ([68a018b](https://github.com/bingling-sama/eidolon/commit/68a018b704796e4bc51c56acc18f9a43a22207c1))

<!-- generated by git-cliff -->
