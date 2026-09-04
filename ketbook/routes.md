# Swap and bridge

We pull LI.FI cheapest (and fastest on bridges), skip their sim, **RPC-sim the top paths in parallel**, and pick the **best user net**: full output + `$VAPURR` refund − gas. Quotes cache 8s. We do not cut the route. A fatter quote that reverts loses to a path that actually sims.

Protocol **25 bps** buys `$VAPURR`. You get a **small `$VAPURR` refund** (5 bps). The rest burns to mint `$PUSD`.

A route is payable only after we **simulate the tx on RPC** (`eth_call` + gas) and draw the trace. Sign on this device broadcasts that tx. `$PUSD` spend stays testnet. Scan opens in-window for Robinhood Chain.

USDG may appear on the quote because that is the chain’s dollar. The mint is still burn `$VAPURR` → `$PUSD`.
