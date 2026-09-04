// Pulled from https://github.com/terra-money/classic-core/blob/main/x/market/keeper/swap.go
// Do not edit to "improve". The EVM port is ../PusdMarket.sol.
package keeper

import (
	sdk "github.com/cosmos/cosmos-sdk/types"

	core "github.com/classic-terra/core/types"
	"github.com/classic-terra/core/x/market/types"

	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
)

// ApplySwapToPool updates each pool with offerCoin and askCoin taken from swap operation,
// OfferPool = OfferPool + offerAmt (Fills the swap pool with offerAmt)
// AskPool = AskPool - askAmt       (Uses askAmt from the swap pool)
func (k Keeper) ApplySwapToPool(ctx sdk.Context, offerCoin sdk.Coin, askCoin sdk.DecCoin) error {
	// No delta update in case Terra to Terra swap
	if offerCoin.Denom != core.MicroLunaDenom && askCoin.Denom != core.MicroLunaDenom {
		return nil
	}

	terraPoolDelta := k.GetTerraPoolDelta(ctx)

	// In case swapping Terra to Luna, the terra swap pool(offer) must be increased and the luna swap pool(ask) must be decreased
	if offerCoin.Denom != core.MicroLunaDenom && askCoin.Denom == core.MicroLunaDenom {
		offerBaseCoin, err := k.ComputeInternalSwap(ctx, sdk.NewDecCoinFromCoin(offerCoin), core.MicroSDRDenom)
		if err != nil {
			return err
		}

		terraPoolDelta = terraPoolDelta.Add(offerBaseCoin.Amount)
	}

	// In case swapping Luna to Terra, the luna swap pool(offer) must be increased and the terra swap pool(ask) must be decreased
	if offerCoin.Denom == core.MicroLunaDenom && askCoin.Denom != core.MicroLunaDenom {
		askBaseCoin, err := k.ComputeInternalSwap(ctx, askCoin, core.MicroSDRDenom)
		if err != nil {
			return err
		}

		terraPoolDelta = terraPoolDelta.Sub(askBaseCoin.Amount)
	}

	k.SetTerraPoolDelta(ctx, terraPoolDelta)

	return nil
}

// ComputeSwap returns the amount of asked coins should be returned for a given offerCoin at the effective
// exchange rate registered with the oracle.
func (k Keeper) ComputeSwap(ctx sdk.Context, offerCoin sdk.Coin, askDenom string) (retDecCoin sdk.DecCoin, spread sdk.Dec, err error) {
	if offerCoin.Denom == askDenom {
		return sdk.DecCoin{}, sdk.ZeroDec(), sdkerrors.Wrap(types.ErrRecursiveSwap, askDenom)
	}

	baseOfferDecCoin, err := k.ComputeInternalSwap(ctx, sdk.NewDecCoinFromCoin(offerCoin), core.MicroSDRDenom)
	if err != nil {
		return sdk.DecCoin{}, sdk.Dec{}, err
	}

	retDecCoin, err = k.ComputeInternalSwap(ctx, baseOfferDecCoin, askDenom)
	if err != nil {
		return sdk.DecCoin{}, sdk.Dec{}, err
	}

	if offerCoin.Denom != core.MicroLunaDenom && askDenom != core.MicroLunaDenom {
		var tobinTax sdk.Dec
		offerTobinTax, err2 := k.OracleKeeper.GetTobinTax(ctx, offerCoin.Denom)
		if err2 != nil {
			return sdk.DecCoin{}, sdk.Dec{}, err2
		}

		askTobinTax, err2 := k.OracleKeeper.GetTobinTax(ctx, askDenom)
		if err2 != nil {
			return sdk.DecCoin{}, sdk.Dec{}, err2
		}

		if askTobinTax.GT(offerTobinTax) {
			tobinTax = askTobinTax
		} else {
			tobinTax = offerTobinTax
		}

		spread = tobinTax
		return
	}

	basePool := k.BasePool(ctx)
	minSpread := k.MinStabilitySpread(ctx)

	cp := basePool.Mul(basePool)
	terraPoolDelta := k.GetTerraPoolDelta(ctx)
	terraPool := basePool.Add(terraPoolDelta)
	lunaPool := cp.Quo(terraPool)

	var offerPool sdk.Dec
	var askPool sdk.Dec
	if offerCoin.Denom != core.MicroLunaDenom {
		offerPool = terraPool
		askPool = lunaPool
	} else {
		offerPool = lunaPool
		askPool = terraPool
	}

	askBaseAmount := askPool.Sub(cp.Quo(offerPool.Add(baseOfferDecCoin.Amount)))

	baseOfferAmount := baseOfferDecCoin.Amount
	spread = baseOfferAmount.Sub(askBaseAmount).Quo(baseOfferAmount)

	if spread.LT(minSpread) {
		spread = minSpread
	}

	return retDecCoin, spread, nil
}

func (k Keeper) ComputeInternalSwap(ctx sdk.Context, offerCoin sdk.DecCoin, askDenom string) (sdk.DecCoin, error) {
	if offerCoin.Denom == askDenom {
		return offerCoin, nil
	}

	offerRate, err := k.OracleKeeper.GetLunaExchangeRate(ctx, offerCoin.Denom)
	if err != nil {
		return sdk.DecCoin{}, sdkerrors.Wrap(types.ErrNoEffectivePrice, offerCoin.Denom)
	}

	askRate, err := k.OracleKeeper.GetLunaExchangeRate(ctx, askDenom)
	if err != nil {
		return sdk.DecCoin{}, sdkerrors.Wrap(types.ErrNoEffectivePrice, askDenom)
	}

	retAmount := offerCoin.Amount.Mul(askRate).Quo(offerRate)
	if retAmount.LTE(sdk.ZeroDec()) {
		return sdk.DecCoin{}, sdkerrors.Wrap(sdkerrors.ErrInvalidCoins, offerCoin.String())
	}

	return sdk.NewDecCoinFromDec(askDenom, retAmount), nil
}
