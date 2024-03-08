import z3

s = z3.Solver()
MAX = 255
DEPOSIT = 0
WITHDRAW = 1
MINT = 2
BURN = 3
BORROW = 4
REPAY = 5
LIQUIDATE = 6

assumptions = []
callers = []
collaterals = []
shares = []
bonds = []
callers = []

ints = 'total_collateral0 collateral0 last_total_liquidity0 total_shares0 shares0 total_borrowable0 total_bonds0 bonds0 caller0 action0 x0 interest0'

def f(iter, total_collateral0, collateral0, last_total_liquidity0, total_shares0, shares0, total_borrowable0, total_bonds0, bonds0):
    if not collaterals:
        collaterals.append(collateral0)
    if not shares:
        shares.append(shares0)
    if not bonds:
        bonds.append(bonds0)

    ints1 = ints.replace('0', iter)
    total_collateral1, collateral1, last_total_liquidity1, total_shares1, shares1, total_borrowable1, total_bonds1, bonds1, caller1, action1, x1, interest1 = z3.Ints(ints1)

    s.add(x1 >= 0, x1 <= MAX)
    s.add(interest1 >= 0, interest1 <= MAX)

    total_liquidity1 = last_total_liquidity0 + interest1
    s.add(total_liquidity1 <= MAX)

    collateral_delta1 = z3.If(action1 == DEPOSIT, x1, z3.If(action1 == WITHDRAW, -x1, 0))

    s.add(total_collateral1 == total_collateral0 + collateral_delta1)
    s.add(z3.Implies(action1 == DEPOSIT, total_collateral1 <= MAX)) # let new_total_collateral = self.total_collateral.checked_add(to_deposit).ok_or(LAssetError::DepositOverflow)?;
    assumptions.append(total_collateral1 < 0) # let new_total_collateral = sub(self.total_collateral, to_withdraw);

    last_collateral1 = collaterals[0]
    for call, coll in enumerate(collaterals[1:]):
        last_collateral1 = z3.If(caller1 == call, coll, last_collateral1)

    s.add(collateral1 == last_collateral1 + collateral_delta1)
    s.add(z3.Implies(action1 == WITHDRAW, collateral1 >= 0)) # let new_collateral = collateral.checked_sub(to_withdraw).ok_or(LAssetError::WithdrawOverflow)?;
    assumptions.append(collateral1 > MAX) # let new_collateral = add(collateral, to_deposit);

    last_shares1 = shares[0]
    for call, share in enumerate(shares[1:]):
        last_shares1 = z3.If(caller1 == call, share, last_shares1)

    liquidity_from_shares1 = z3.If(action1 == BURN, z3.If(total_shares0 == 0, 0, x1 * total_liquidity1 / total_shares0), 0)
    assumptions.append(liquidity_from_shares1 > MAX) # let to_withdraw = mulw(to_burn, total_liquidity).div_rate(total_shares).unwrap_or(0);

    liquidity_delta1 = z3.If(action1 == MINT, x1, -liquidity_from_shares1)
    s.add(last_total_liquidity1 == total_liquidity1 + liquidity_delta1)

    s.add(z3.Implies(action1 == MINT, last_total_liquidity1 <= MAX)) # let new_total_liquidity = total_liquidity.checked_add(to_wrap).ok_or(LAssetError::MintLiquidityOverflow)?;
    assumptions.append(last_total_liquidity1 < 0) # let new_total_liquidity = sub(total_liquidity, to_withdraw);

    shares_from_liquidity1 = z3.If(action1 == MINT, z3.If(total_liquidity1 == 0, x1, x1 * total_shares0 / total_liquidity1), 0)
    assumptions.append(shares_from_liquidity1 > MAX) # let to_mint = mulw(to_wrap, total_shares).div_rate(total_liquidity).unwrap_or(to_wrap);

    share_delta1 = z3.If(action1 == BURN, -x1, shares_from_liquidity1)
    s.add(total_shares1 == total_shares0 + share_delta1)
    assumptions.append(total_shares1 < 0)
    assumptions.append(total_shares1 > MAX) # let new_total_shares = add(total_shares, to_mint);

    s.add(shares1 == last_shares1 + share_delta1)
    s.add(z3.Implies(action1 == BURN, shares1 >= 0)) # let new_shares = shares.checked_sub(to_burn).ok_or(LAssetError::BurnOverflow)?;
    assumptions.append(shares1 < 0) # let new_total_shares = sub(total_shares, to_burn);
    assumptions.append(shares1 > MAX) # let new_shares = add(shares, to_mint);

    total_debt1 = total_liquidity1 - total_borrowable0
    assumptions.append(total_debt1 < 0) # let total_debt = sub(total_liquidity, total_borrowable);
    bonds_from_debt1 = z3.If(action1 == BORROW, z3.If(total_debt1 == 0, x1, (x1 * total_bonds0 + total_debt1 - 1) / total_debt1), 0)
    assumptions.append(bonds_from_debt1 > MAX) # let to_mint = mulw(to_borrow, total_bonds).ceil_rate(total_debt).unwrap_or(to_borrow);

    bonds_delta1 = z3.If(action1 == REPAY, -x1, bonds_from_debt1)
    s.add(total_bonds1 == total_bonds0 + bonds_delta1)
    assumptions.append(total_bonds1 > MAX) # let new_total_bonds = add(total_bonds, to_mint); 
    assumptions.append(total_bonds1 < 0) # let new_total_bonds = sub(total_bonds, to_repay);

    last_bonds1 = bonds[0]
    for call, bond in enumerate(bonds[1:]):
        last_bonds1 = z3.If(caller1 == call, bond, last_bonds1)

    s.add(bonds1 == last_bonds1 + bonds_delta1)
    s.add(z3.Implies(action1 == REPAY, bonds1 >= 0))
    assumptions.append(bonds1 > MAX) # let new_bonds = add(bonds, to_mint);

    borrowable_from_bonds1 = z3.If(action1 == REPAY, z3.If(total_bonds0 == 0, total_debt1, (x1 * total_debt1 + total_bonds0 - 1) / total_bonds0), 0)
    assumptions.append(borrowable_from_bonds1 > MAX) # let repaid = mulw(to_repay, total_debt).ceil_rate(total_bonds).unwrap_or(total_debt);

    borrowable_delta1 = z3.If(action1 == BORROW, -x1, z3.If(action1 == REPAY, borrowable_from_bonds1, liquidity_delta1))
    s.add(total_borrowable1 == total_borrowable0 + borrowable_delta1)
    s.add(z3.Implies(action1 == BORROW, total_borrowable1 >= 0)) # let new_total_borrowable = total_borrowable.checked_sub(to_borrow).ok_or(LAssetError::BorrowOverflow)?;
    s.add(z3.Implies(action1 == BURN, total_borrowable1 >= 0)) # let new_total_borrowable = total_borrowable.checked_sub(to_withdraw).ok_or(LAssetError::BurnTooMuch)?;
    assumptions.append(total_borrowable1 > MAX) # let new_total_borrowable = add(total_borrowable, to_wrap); let new_total_borrowable = add(total_borrowable, repaid);

    total_new_debt = last_total_liquidity1 - total_borrowable1
    assumptions.append(total_new_debt < 0) # let new_total_debt = sub(total_liquidity, new_total_borrowable);
    new_debt1 = z3.If(total_bonds1 == 0, total_new_debt, (bonds1 * total_new_debt + total_bonds1 - 1) / total_bonds1)
    assumptions.append(new_debt1 > MAX) # let debt = mulw(new_bonds, new_total_debt).ceil_rate(new_total_bonds).unwrap_or(new_total_debt);

    old_debt1 = new_debt1 + borrowable_from_bonds1
    assumptions.append(old_debt1 > MAX) # let old_debt = add(debt, repaid);

    collaterals.append(collateral1)
    shares.append(shares1)
    bonds.append(bonds1)
    callers.append(caller1)
    return total_collateral1, collateral1, last_total_liquidity1, total_shares1, shares1, total_borrowable1, total_bonds1, bonds1

total_collateral0, collateral0, last_total_liquidity0, total_shares0, shares0, total_borrowable0, total_bonds0, bonds0, _, _, _, _ = z3.Ints(ints)
s.add(total_collateral0 == 0, collateral0 == 0, last_total_liquidity0 == 0, total_shares0 == 0, shares0 == 0, total_borrowable0 == 0, total_bonds0 == 0, bonds0 == 0)

total_collateral1, collateral1, last_total_liquidity1, total_shares1, shares1, total_borrowable1, total_bonds1, bonds1 = f('1', total_collateral0, collateral0, last_total_liquidity0, total_shares0, shares0, total_borrowable0, total_bonds0, bonds0)
total_collateral2, collateral2, last_total_liquidity2, total_shares2, shares2, total_borrowable2, total_bonds2, bonds2 = f('2', total_collateral1, collateral1, last_total_liquidity1, total_shares1, shares1, total_borrowable1, total_bonds1, bonds1)
total_collateral3, collateral3, last_total_liquidity3, total_shares3, shares3, total_borrowable3, total_bonds3, bonds3 = f('3', total_collateral2, collateral2, last_total_liquidity2, total_shares2, shares2, total_borrowable2, total_bonds2, bonds2)

s.add(z3.Distinct(*callers))

for a in assumptions:
    print('checking...', a)
    if s.check(a) == z3.unsat:
        print("OK")
    else:
        m = s.model()
        print("FAILED", m)
