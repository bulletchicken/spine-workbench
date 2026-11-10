//! Every random draw comes from the recipe seed, through our own generator:
//! SplitMix64 seeding xoshiro256**, one stream per purpose, named by an FNV-1a
//! hash of its tag. No random-number crate is used, so the streams never shift
//! when a dependency changes, and adding a feature never moves another
//! feature's numbers.
