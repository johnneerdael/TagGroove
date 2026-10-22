# Cloudflare solver

Some music sites sit behind Cloudflare and, now and then, answer TagGroove with a
"checking your browser" challenge or a block page instead of the page you
asked for. A **Cloudflare solver** is a small service you run that opens such a
page in a real browser and hands back the result. TagGroove can use any service
that speaks the **FlareSolverr** API, so you can pick the one you prefer:

- [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr), the original
- [Byparr](https://github.com/ThePhaseless/Byparr), a drop-in replacement
- trawl, another service with the same API

TagGroove does not install, start or bundle a solver, and most people never need
one. Traxsource is the source that asks for one most often; the setting applies
to every tag source.

## Set one up

Choose **Tag Sources → Options…** and open **Cloudflare solver**.

<!-- figure:options-tag-sources-solver -->
<figure markdown>
![The Cloudflare solver page of the Options dialog with the solver endpoint, timeout and the Retry direct access button](../assets/screens/options-tag-sources-solver-light.png#only-light)
![The Cloudflare solver page of the Options dialog with the solver endpoint, timeout and the Retry direct access button](../assets/screens/options-tag-sources-solver-dark.png#only-dark)
<figcaption>Options → Tag Sources → Cloudflare solver holds the optional solver endpoint.</figcaption>
</figure>
<!-- /figure -->

- **Solver endpoint:** the address of your solver, for example
  `http://127.0.0.1:8191`. TagGroove adds `/v1` when you leave it off, so
  `http://127.0.0.1:8191` and `http://127.0.0.1:8191/v1` are the same. The address
  needs a host and no user name, password, query parameters or fragment. Leave
  the box empty to use no solver.
- **Solver timeout (seconds):** how long to wait for the solver, from 1 to 300.
  The default is 120.
- **Retry direct access:** see [When a site blocks you](#when-a-site-blocks-you).

Choose **OK** to save. There is nothing to restart.

## When a site blocks you

TagGroove recognises Cloudflare's own signals: its challenge header, its
challenge page and its block pages. It does not treat an ordinary "not found" or
a server error as a block.

When it sees one:

1. TagGroove repeats that request through your solver, whatever the request was:
   a search, a release page, a track page.
2. From then on in this session it stops contacting that site directly. Every
   later request to the same site goes through the solver, so a site that has
   already blocked you is not hit again with requests that are bound to be
   refused.
3. With no solver set up, the lookup fails and the message tells you to add one
   in **Options → Tag Sources → Cloudflare solver**. Nothing is saved or changed
   in your files.

Choose **Retry direct access** after you fixed the solver or the connection, or
when you think the block has passed. It makes TagGroove forget which sites
blocked it, so the next request goes directly again. It does not change any tags.
Restarting TagGroove has the same effect, because the memory lasts one session.

An unreachable or rejecting solver produces a lookup error. A challenge page that
comes back is never treated as an empty result.

## What the solver sees

The solver receives the address of each request that is sent through it, and the
page that comes back. Use a solver on a machine you trust, and keep its address
free of passwords.

TagGroove never sends a request that carries a credential to a solver. That covers
your Discogs token, API keys, the access token of Beatport's catalog API, signed
requests and any address with a key, token or signature in it, as well as form
submissions and image downloads. If such a request is blocked, the lookup
fails with a message that says so, rather than passing the secret on. The solver
also never receives your tags or files.

## Related

- [Beatport and Traxsource](catalog.md)
- [Network access, privacy and rate limits](privacy-and-network.md)
- [Tag Sources options](../reference/options/tag-sources.md#cloudflare-solver)
