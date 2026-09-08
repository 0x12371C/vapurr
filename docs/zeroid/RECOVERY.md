# zer0ID wallet recovery — design

**Status:** core implemented (`crates/vapurr-id/src/recovery.rs`), factors not yet sealed.
**Goal:** after KYC, a user who loses the device can recover the wallet.

## The thing that does not work

"Derive the wallet key from the retina" is the obvious design and it is not
buildable as stated.

A biometric read is **noisy**. Every scan of the same eye differs — lighting,
angle, pupil dilation, occlusion. open-iris produces a template plus a *match
score*, not a stable value. A private key cannot tolerate a single flipped bit,
so raw biometric output is never key material.

Making a biometric reproduce a stable key requires a **fuzzy extractor** (a
secure sketch plus error correction). Its defining property is that it stores
**helper data** — public-ish bytes, kept per user, that let a close-enough scan
reproduce the enrolled key.

That collides head-on with what zer0ID currently promises on `vapurr://id` and
on thesecretlab.app:

> Frames are deleted right after inference, and the iris template itself is
> never written to disk — only an opaque nullifier (a one-way hash) is kept.

A one-way nullifier reconstructs nothing. That is the point of it. **You can
have irreversibility or biometric recovery. Not both.** Shipping biometric
recovery means persisting helper data per user, and the privacy copy has to
change to say so. This is a product decision, not an implementation detail.

Two further properties make the biometric a poor *sole* factor:

- **It cannot be rotated.** A leaked password is changed in a minute. A leaked
  iris helper-data set burns that factor permanently, for that human, forever.
- **Effective entropy is well below the theoretical figure.** Error correction
  spends entropy, and helper data leaks some. Iris fuzzy extractors are a
  research-grade build, not a wiring task.

## The shape that does work

Factors **release shares**. They are never themselves the key.

1. The seed (or a key-encrypting key over it) is split with Shamir over
   GF(2^8): `n` shares, any `t` reconstruct.
2. Each share is sealed to one factor.
3. Recovery gathers any `t` factors. Below `t`, Shamir is
   information-theoretically secure — `t-1` shares leave every secret equally
   likely. Not "computationally hard": *no information*.

This is factor-agnostic on purpose. The Shamir core is correct and testable
today; it does not depend on the fuzzy extractor existing.

## Why not 1-of-2 ("SMS or retina")

The request was recovery by SMS **or** retina. That is 1-of-2, and it means
**either factor alone moves the wallet**. Concretely, today:

- **SMS** is SIM-swappable — a well-documented way people lose custodial-ish
  accounts. Worse, zer0ID level 3 currently falls back to an **on-screen mock
  code** when carrier SMS is unwired (see `RHC.md`). Under 1-of-2, "recover by
  SMS" would then mean *anyone who can load the page* takes the wallet.
- **Retina**, once helper data leaks, is compromised permanently and cannot be
  reissued.

`RecoveryPolicy::new` and `split_with` both refuse `threshold < 2`. This is
enforced in code, not documented as a guideline.

**Recommended:** 2-of-3 over {phone, biometric, backup code}, with the device
key as a convenience fourth that skips recovery entirely while the device
lives. Any two independent factors recover; no single factor — and no single
operator — can.

## Factors

| Factor | What it proves | Weakness | Built |
|---|---|---|---|
| `Device` | Possession of this install | Lost with the device | yes |
| `Phone` | Control of a number | SIM swap; **mock-code fallback today proves nothing** | yes |
| `Biometric` | One specific human | Needs stored helper data; unrotatable | **no** |
| `BackupCode` | Possession of the written code | User loses paper | yes |

`Factor::implemented()` returns false for `Biometric`, and
`RecoveryPolicy::enactable_today()` is false for any policy that cannot meet
its threshold without it. Nothing silently pretends the biometric works.

## Operator trust

Server-held shares must be sealed so the operator cannot unilaterally
reconstruct — a share released by the phone factor should be encrypted under a
key the server does not hold. Otherwise the recovery service *is* custody, and
the wallet stops being self-custodial regardless of what the UI says.

## Where ZK genuinely applies

ZK proves a statement without revealing the witness. It does not store or
reconstruct secrets, so it is not the recovery mechanism. It is the right tool
for the *attestation*: prove "a unique human passed KYC" without revealing who
— which is what the nullifier already does. Calling recovery "zk" would be a
misnomer; describing the attestation that way is fair.

## Built vs not built

- **Built:** Shamir split/combine over GF(2^8), threshold enforcement,
  policy model, factor capability reporting. Algorithm independently validated
  by executing an equivalent transcription: all 255 inverses round-trip, every
  3-of-5 subset reconstructs, below-threshold does not, 32-byte seeds recover
  from any qualifying pair.
- **Not built:** sealing shares to factors, the fuzzy extractor, server-side
  share custody, the recovery UI, and re-enrolment/rotation.

## Open decisions

1. Does the privacy promise change to allow stored helper data? Nothing
   biometric can be recovered until this is answered.
2. Threshold and factor set — 2-of-3 recommended over the requested 1-of-2.
3. Who holds the phone-released share, and under what key, so recovery does not
   quietly become custody.
