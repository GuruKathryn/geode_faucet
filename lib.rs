/*
ABOUT THIS CONTRACT...
This contract allows users to get a little coin for their friends who they have 
helped sign up. It also sends a little coin to the user for helping their friend. 
*/

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod geode_faucet {

    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    // PRELIMINARY DATA STRUCTURES >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout,))]
    pub struct Pebble {
        timestamp: u64,
        ip_address: Vec<u8>,
        pebble: AccountId,
        payout: Balance,
    }
    
    impl Default for Pebble {
        fn default() -> Pebble {
            Pebble {
                timestamp: u64::default(),
                ip_address: <Vec<u8>>::default(),
                pebble: AccountId::from([0x0; 32]),
                payout: Balance::default(),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Default)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout,))]
    pub struct ViewStats { 
        eligible_payout: Balance,
        get_payout: Balance,
        limit_timer: u64,
        limit_ip_total: u128,
        total_pebble_accounts: u128,
        total_payouts: Balance,
    }


    // EVENT DEFINITIONS >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[ink(event)]
    // writes a new payout to the chain. 
    pub struct PayoutEvent {
        #[ink(topic)]
        timestamp: u64,
        #[ink(topic)]
        user_ip: Vec<u8>,
        #[ink(topic)]
        pebble: AccountId,
        payout: Balance,
    }


    // ERROR DEFINITIONS >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    // Errors that can occur upon calling this contract
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        // denies permission for root only actions
        PermissionDenied,
        // pauout failed to go through
        PayoutFailed,
    }


    // ACTUAL CONTRACT STORAGE >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
    #[ink(storage)]
    pub struct ContractStorage {
        user_map: Mapping<AccountId, Pebble>,
        ipaddress_count: Mapping<Vec<u8>, Vec<AccountId>>,
        root: AccountId,
        rootset: u8,
        eligible_payout: Balance,
        get_payout: Balance,
        limit_timer: u64,
        limit_ip_total: u128,
        total_pebble_accounts: u128, 
        total_payouts: Balance,
    }

    // CONTRACT LOGIC >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    impl ContractStorage {
        
        // CONSTRUCTORS >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
        // Constructors are implicitly payable when the contract is instantiated.

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                user_map: Mapping::default(),
                ipaddress_count: Mapping::default(),
                root: AccountId::from([0x0; 32]),
                rootset: 0,
                eligible_payout: 0,
                get_payout: 0,
                limit_timer: u64::default(),
                limit_ip_total: u128::default(),
                total_pebble_accounts: u128::default(), 
                total_payouts: Balance::default(),
            }
        }


        // >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
        // MESSGE FUNCTIONS THAT ALTER CONTRACT STORAGE >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
        // >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
        
        // 0 🟢 SET ROOT ACCOUNT
        // This message lets us set the root account 
        #[ink(message)]
        pub fn set_root_account(&mut self, 
            new_root: AccountId,
        ) -> Result<(), Error> {
            let caller = Self::env().caller();
            // check that the root user is not yet set
            if self.rootset != 1 || self.root == caller {
                // proceed to set up the root user
                self.root = new_root;
                self.rootset = 1;
            }
            else {
                // if the root user has already been set 
                // and the caller is not that root user, send an error
                return Err(Error::PermissionDenied)
            }
            Ok(())
        }


        // 1 🟢 SET PAYOUTS AND LIMITS & SEND COIN (ROOT ONLY)
        // This message lets the root account set/update payouts and limits and send coin
        #[ink(message, payable)]
        pub fn set_payouts_and_fund(&mut self, 
            new_eligible_payout: Balance,
            new_get_payout: Balance,
            new_limit_timer: u64,
            new_limit_ip_total: u128
        ) -> Result<(), Error> {
            // check that the caller is the root user
            let caller = Self::env().caller();
            if self.root == caller {
                // set all the things
                self.eligible_payout = new_eligible_payout;
                self.get_payout = new_get_payout;
                self.limit_timer = new_limit_timer;
                self.limit_ip_total = new_limit_ip_total;
            }
            // if the caller is not the root user, return fail
            else {
                // error
                return Err(Error::PermissionDenied)
            }
            Ok(())
        }


        // 2 🟢 CHECK ELIGIBILITY [ANYONE]
        // lets any one user check if they are eligible to get coin
        // if eligible, it transfers the eligible_payout to their account
        #[ink(message)]
        pub fn check_eligibility(&self, my_ip_address: Vec<u8>) -> u8 {
            let caller = Self::env().caller();
            let mut result: u8 = 0;

            let now = self.env().block_timestamp();
            let user_details = self.user_map.get(caller).unwrap_or_default();
            let time_since = now.wrapping_sub(user_details.timestamp);
            let ip_tags = self.ipaddress_count.get(my_ip_address.clone()).unwrap_or_default();
            let ip_tags_len: u128 = ip_tags.len().try_into().unwrap_or_default();

            // return YES if...
            // the IP address has < the limit of total IP tags AND
            // EITHER the user has not paid out before OR...
            // the user has paid out before but it has been long enough
            if (ip_tags_len < self.limit_ip_total || ip_tags.contains(&caller)) 
            && (time_since >= self.limit_timer || user_details.payout == 0) {
                // change results to yes
                result = 1;

                // payout the eligible_payout to the caller
                // make sure the contract has enough balance
                if self.env().balance() > self.eligible_payout {
                    if self.env().transfer(caller, self.eligible_payout).is_err() {
                        result = 2;
                    }
                }
                
                // emit and event for the payout
                Self::env().emit_event(PayoutEvent {
                    timestamp: now,
                    user_ip: my_ip_address,
                    pebble: caller,
                    payout: self.eligible_payout,
                });
            }

            // return result (yes or no)
            result
        }


        // 3 🟢 GET COIN [ANYONE]
        // lets any one user who is eligible, get coin from the faucet
        #[ink(message)]
        pub fn get_coin(&mut self, 
            my_ip_address: Vec<u8>
        ) -> Result<(), Error> {
            let caller = Self::env().caller();
            let now = self.env().block_timestamp();

            let mut newuser: u8 = 1;
            if self.user_map.contains(&caller) {
                newuser = 0;
            }

            let mut user_details = self.user_map.get(caller).unwrap_or_default();
            let time_since = now.wrapping_sub(user_details.timestamp);
            let mut ip_tags = self.ipaddress_count.get(my_ip_address.clone()).unwrap_or_default();
            let ip_tags_len: u128 = ip_tags.len().try_into().unwrap_or_default();

            // account is eligible if...
            // the IP address has < the limit of total IP tags AND
            // EITHER the user has not paid out before OR...
            // the user has paid out before but it has been long enough
            if (ip_tags_len < self.limit_ip_total || ip_tags.contains(&caller)) 
            && (time_since >= self.limit_timer || user_details.payout < 1) {

                // payout the get_payout to the caller
                // make sure the contract has enough balance
                if self.env().balance() > self.get_payout {
                    if self.env().transfer(caller, self.get_payout).is_err() {
                        return Err(Error::PayoutFailed);
                    }
                }
                
                // update the user details (timestamp updated on get coin only)
                user_details.payout = user_details.payout.saturating_add(self.get_payout);
                user_details.ip_address = my_ip_address.clone();
                user_details.pebble = caller;
                user_details.timestamp = now;
                
                // update the user_map
                self.user_map.insert(caller, &user_details);

                // update the ip address count 
                if ip_tags.contains(&caller) {
                    // do nothing
                }
                else {
                    // add the caller and update the map
                    ip_tags.push(caller);
                    self.ipaddress_count.insert(my_ip_address.clone(), &ip_tags);
                }

                // update total total_payouts
                self.total_payouts = self.total_payouts.saturating_add(self.get_payout);

                // update the total_pebble_accounts IF this is a new account
                if newuser == 1 {
                    self.total_pebble_accounts = self.total_pebble_accounts.saturating_add(1);
                }
                
                // emit event for the payout
                Self::env().emit_event(PayoutEvent {
                    timestamp: now,
                    user_ip: my_ip_address,
                    pebble: caller,
                    payout: self.get_payout,
                });

            }
            else {
                // send error that permission is denied
                return Err(Error::PermissionDenied);
            }

            Ok(())
        }


        // 4 🟢 GET STATS AND SETTINGS DATA
        // returns all the pertinent stats and settings data in storage
        #[ink(message)]
        pub fn get_stats_and_settings(&self) -> ViewStats {
            let stats = ViewStats {
                eligible_payout: self.eligible_payout,
                get_payout: self.get_payout,
                limit_timer: self.limit_timer,
                limit_ip_total: self.limit_ip_total,
                total_pebble_accounts: self.total_pebble_accounts,
                total_payouts: self.total_payouts,
            };
            // return results
            stats
        }


        // 5 🟢 VERIFY ACCOUNT 
        // for use in other apps, returns 1 if the account has tagged the faucet at least once
        #[ink(message)]
        pub fn verify_account(&self, verify: AccountId) -> u8 {
            let mut result: u8 = 0;
            if self.user_map.contains(&verify) {
                result = 1;
            }
            //return result
            result
        }


        // END OF MESSAGE FUNCTIONS

    }
    // END OF CONTRACT LOGIC

}
