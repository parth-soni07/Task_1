use ic_cdk::export::candid::{CandidType, Deserialize};
use std::collections::{HashMap, HashSet};
use ic_cdk::export::Principal;

#[derive(CandidType, Deserialize, Clone)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub total_supply: u64,
    pub owner: Principal,
    pub decimals: u8,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct TransactionRecord {
    pub from: Principal,
    pub to: Principal,
    pub amount: u64,
    pub post_balance_from: u64,
    pub post_balance_to: u64,
    pub cycles_burnt: u64,
    pub reason: String,
}
pub struct TokenICRC2 {
    balances: HashMap<Principal, u64>,
    allowances: HashMap<Principal, HashMap<Principal, u64>>,
    minters: HashSet<Principal>, 
    owner: Principal,
    total_supply: u64,
    decimals: u8,
    name: String,
    symbol: String,
    burnt_cycles: u64,
    transaction_history: Vec<TransactionRecord>,

}

impl TokenICRC2 {
    pub fn new(owner: Principal, total_supply: u64, decimals: u8, name: String, symbol: String) -> Self {
        let mut balances = HashMap::new();
        let mut minters = HashSet::new();
        balances.insert(owner, total_supply);
        minters.insert(owner);  // Owner starts as the initial minter
        Self {
            balances,
            allowances: HashMap::new(),
            minters,
            owner,
            total_supply,
            decimals,
            name,
            symbol,
            burnt_cycles: 0,
            transaction_history: Vec::new(),

        }
    }
    pub fn get_owner(&self) -> Principal {
        self.owner.clone()
    }
    pub fn balance_of(&self, user: Principal) -> u64 {
        *self.balances.get(&user).unwrap_or(&0)
    }

    pub fn allowance(&self, owner: Principal, spender: Principal) -> u64 {
        self.allowances
            .get(&owner)
            .and_then(|spenders| spenders.get(&spender))
            .copied()
            .unwrap_or(0)
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn symbol(&self) -> String {
        self.symbol.clone()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn transfer(&mut self, from: Principal, to: Principal, amount: u64) -> Result<(), String> {
        let from_balance = self.balances.get(&from).unwrap_or(&0);
        if *from_balance < amount {
            return Err("Insufficient balance".to_string());
        }
        *self.balances.entry(from).or_insert(0) -= amount;
        *self.balances.entry(to).or_insert(0) += amount;
        // Check for cycles burnt
        let cycles_burnt = self.burnt_cycles; // assuming burnt_cycles represents the most recent burn
        let reason = if cycles_burnt > 0 {
            "Cycles were burnt due to transfer fees or maintenance costs.".to_string()
        } else {
            "No cycles were burnt as no transfer fees applied.".to_string()
        };

        // Log the transaction
        let record = TransactionRecord {
            from,
            to,
            amount,
            post_balance_from: self.balances.get(&from).copied().unwrap_or(0),
            post_balance_to: self.balances.get(&to).copied().unwrap_or(0),
            cycles_burnt,
            reason,
        };
        self.transaction_history.push(record);

        Ok(())
    }

    pub fn approve(&mut self, owner: Principal, spender: Principal, amount: u64) -> Result<(), String> {
        self.allowances
            .entry(owner)
            .or_insert_with(HashMap::new)
            .insert(spender, amount);
        Ok(())
    }

    pub fn burn_cycles(&mut self, cycles: u64) {
        self.burnt_cycles += cycles;
    }

    pub fn burnt_cycles(&self) -> u64 {
        self.burnt_cycles
    }
    pub fn add_minter(&mut self, minter: Principal) -> Result<(), String> {
        let owner = ic_cdk::caller();
        if owner != self.get_owner() {
            return Err("Only the owner can add minters".to_string());
        }
        self.minters.insert(minter);
        Ok(())
    }

    pub fn mint(&mut self, to: Principal, amount: u64) -> Result<(), String> {
        let caller = ic_cdk::caller();
        if !self.minters.contains(&caller) {
            return Err("Caller is not authorized to mint".to_string());
        }
        *self.balances.entry(to).or_insert(0) += amount;
        self.total_supply += amount;
        let record = TransactionRecord {
            from: caller,
            to,
            amount,
            post_balance_from: 0,
            post_balance_to: self.balances.get(&to).copied().unwrap_or(0),
            cycles_burnt: 0,
            reason: "Minting operation has no cycle burn cost.".to_string(),
        };
        self.transaction_history.push(record);
        Ok(())
    }
    pub fn get_transaction_history(&self) -> Vec<TransactionRecord> {
        self.transaction_history.clone()
    }
}

thread_local! {
    static TOKEN_ICRC2: std::cell::RefCell<Option<TokenICRC2>> = std::cell::RefCell::new(None);
}

#[ic_cdk_macros::update]
fn init_token(symbol: String, name: String, total_supply: u64, decimals: u8) {
    let owner = ic_cdk::caller();
    TOKEN_ICRC2.with(|token| {
        *token.borrow_mut() = Some(TokenICRC2::new(owner, total_supply, decimals, name, symbol));
    });
}
#[ic_cdk_macros::update]
fn add_minter(minter: Principal) -> Result<(), String> {
    TOKEN_ICRC2.with(|token| {
        if let Some(ref mut t) = token.borrow_mut().as_mut() {
            t.add_minter(minter)
        } else {
            Err("Token not initialized".to_string())
        }
    })
}

#[ic_cdk_macros::update]
fn mint(to: Principal, amount: u64) -> Result<(), String> {
    TOKEN_ICRC2.with(|token| {
        if let Some(ref mut t) = token.borrow_mut().as_mut() {
            t.mint(to, amount)
        } else {
            Err("Token not initialized".to_string())
        }
    })
}
#[ic_cdk_macros::query]
fn balance_of(user: Principal) -> u64 {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.balance_of(user)
        } else {
            0
        }
    })
}

#[ic_cdk_macros::query]
fn total_supply() -> u64 {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.total_supply()
        } else {
            0
        }
    })
}

#[ic_cdk_macros::query]
fn symbol() -> String {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.symbol()
        } else {
            "".to_string()
        }
    })
}

#[ic_cdk_macros::query]
fn name() -> String {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.name()
        } else {
            "".to_string()
        }
    })
}

#[ic_cdk_macros::query]
fn decimals() -> u8 {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.decimals()
        } else {
            0
        }
    })
}

#[ic_cdk_macros::query]
fn allowance(owner: Principal, spender: Principal) -> u64 {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.allowance(owner, spender)
        } else {
            0
        }
    })
}

#[ic_cdk_macros::update]
fn approve(spender: Principal, amount: u64) -> Result<(), String> {
    let owner = ic_cdk::caller();
    TOKEN_ICRC2.with(|token| {
        if let Some(ref mut t) = token.borrow_mut().as_mut() {
            t.approve(owner, spender, amount)
        } else {
            Err("Token not initialized".to_string())
        }
    })
}

#[ic_cdk_macros::update]
fn transfer(to: Principal, amount: u64) -> Result<(), String> {
    let from = ic_cdk::caller();
    TOKEN_ICRC2.with(|token| {
        if let Some(ref mut t) = token.borrow_mut().as_mut() {
            t.transfer(from, to, amount)
        } else {
            Err("Token not initialized".to_string())
        }
    })
}

#[ic_cdk_macros::update]
fn burn_cycles(cycles: u64) {
    TOKEN_ICRC2.with(|token| {
        if let Some(ref mut t) = token.borrow_mut().as_mut() {
            t.burn_cycles(cycles);
        }
    });
}

#[ic_cdk_macros::query]
fn burnt_cycles() -> u64 {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.burnt_cycles()
        } else {
            0
        }
    })
}
#[ic_cdk_macros::query]
fn get_transaction_history() -> Vec<TransactionRecord> {
    TOKEN_ICRC2.with(|token| {
        if let Some(t) = token.borrow().as_ref() {
            t.get_transaction_history()
        } else {
            Vec::new()
        }
    })
}
