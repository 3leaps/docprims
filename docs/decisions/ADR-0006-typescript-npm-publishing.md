# ADR-0006: TypeScript npm Publishing Standard

> **Status**: Accepted
> **Date**: 2026-01-31
> **Authors**: releng
> **Supervised by**: @3leapsdave

## Context

docprims provides TypeScript bindings via napi-rs, which compiles to native Node.js addons. Publishing these bindings to npm requires:

1. Building platform-specific `.node` binaries (prebuilds)
2. Publishing multiple scoped packages (one per platform + root package)
3. Authenticating with npm registry

We use OIDC trusted publishing to avoid long-lived npm tokens. This ADR establishes the standard configuration based on learnings from sysprims (ADR-0015) and crucible knowledge base.

## Decision

### 1. OIDC Trusted Publishing (No Tokens)

We use npm OIDC trusted publishing exclusively. **No `NPM_TOKEN` or `NODE_AUTH_TOKEN` secrets shall be stored** in:
- Repository secrets
- Environment secrets
- Organization secrets

Rationale: Any token present will override OIDC, causing authentication failures if the token is expired or revoked.

### 2. npm CLI Version Upgrade

The publish workflow must upgrade npm to >= 11.5.1 before publishing:

```yaml
- name: Ensure npm CLI supports OIDC
  run: |
    npm install -g npm@11.5.1
    echo "npm version: $(npm --version)"
```

Rationale: Ubuntu runners with Node 20 ship npm ~10.x, which lacks OIDC support.

### 3. Force OIDC Mode in Publish Steps

Each publish step must explicitly force OIDC mode:

```yaml
- name: Publish
  run: |
    unset NODE_AUTH_TOKEN NPM_TOKEN
    export NPM_CONFIG_USERCONFIG="$RUNNER_TEMP/npmrc-oidc"
    printf '%s\n' 'registry=https://registry.npmjs.org/' 'always-auth=false' > "$NPM_CONFIG_USERCONFIG"
    npm publish --access public
```

Rationale: `actions/setup-node` with `registry-url` creates `.npmrc` referencing `$NODE_AUTH_TOKEN`. Even without a secret, environment pollution can cause issues.

### 4. GitHub Environment Protection

The publish workflow uses GitHub environment `publish-npm` with:
- Deployment restricted to `v*` tags
- Optional: approval requirements for manual gate

```yaml
jobs:
  publish:
    environment: publish-npm
```

Rationale: Prevents accidental publishes from non-release refs.

### 5. Release Workflow Order

The release process follows this strict order:

```
1. Push all changes to main
2. Verify local/remote sync
3. Run Go Bindings workflow → merge PR
4. Create and push tags (v* and bindings/go/docprims/v*)
5. Run TypeScript N-API Prebuilds from tag
6. Wait for release workflow (builds artifacts)
7. Manual signing + undraft release
8. Run TypeScript npm Publish from tag
```

**Critical constraints:**
- Tags must point to commits that include Go bindings
- Prebuilds must run from tag ref (SHA validation in publish)
- npm publish must run from tag ref (OIDC + environment protection)

### 6. Workflow Files Run from Tag Commit

When publishing from a tag, GitHub uses the workflow YAML from the tagged commit, not from main. This means:

- Workflow fixes pushed to main don't affect tag-triggered runs
- To fix a workflow for an existing tag, the tag must be moved
- Test workflow changes on main before tagging

### 7. First Publish is Manual

OIDC trusted publishing requires each package to already exist on npm. For the first release of any package:

1. Download prebuilds artifacts locally
2. Manually `npm publish --access public` each platform package
3. Manually publish root package
4. Configure trusted publisher on npmjs.com for each package

Subsequent releases use the workflow.

### 8. Supported Platforms

docprims TypeScript bindings support:
- linux-x64-gnu (glibc 2.17+)
- linux-x64-musl (Alpine)
- linux-arm64-gnu (glibc 2.17+)
- linux-arm64-musl (Alpine)
- darwin-arm64 (Apple Silicon)
- win32-x64-msvc (Windows)

Not supported:
- darwin-x64 (Intel Mac) - no demand
- win32-arm64 - no Go bindings either

## Consequences

### Positive

- No long-lived secrets to manage or rotate
- Provenance attestation for supply chain security
- Clear audit trail via GitHub environments
- Reproducible releases via strict ordering

### Negative

- Workflow fixes require tag recreation (moving published tags is risky)
- More complex workflow configuration vs simple token auth
- npm CLI upgrade adds ~10s to workflow runtime
- First release requires manual publish of all packages

### Neutral

- Each platform package needs separate trusted publisher config on npmjs.com
- Environment protection adds manual approval step (can be removed if desired)

## Future Improvements

### Use actions/download-artifact@v4 with run-id

The current workflow uses `gh run download` for artifact retrieval. A cleaner approach would be `actions/download-artifact@v4` with the `run-id` input (fewer moving parts, no gh CLI dependency). This would simplify the retry logic and improve reliability.

## References

- [npm Trusted Publishers Documentation](https://docs.npmjs.com/trusted-publishers/)
- [Crucible Knowledge: npm OIDC](https://crucible.3leaps.dev/knowledge/cicd/registry/npm-oidc)
- [Crucible Knowledge: Workflow Version Resolution](https://crucible.3leaps.dev/knowledge/cicd/github-actions/workflow-version-resolution)
- [sysprims ADR-0015](https://github.com/3leaps/sysprims/blob/main/docs/decisions/ADR-0015-typescript-npm-publishing.md)
- [RELEASE_CHECKLIST.md](../../RELEASE_CHECKLIST.md) - Full release procedure
