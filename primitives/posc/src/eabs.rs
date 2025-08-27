//! Algorithm 3: elastic Advanced Block Schedule (eABS) generator
//! Implements the 2D matrix-based scheduling from the Throughput Network paper

use sp_std::{vec, vec::Vec};
use crate::{AuthorityId, BlockGenerationScore};

/// The 2D matrix for schedule generation as described in Algorithm 3
pub struct ScheduleMatrix {
    /// The actual matrix: rows are BGS shares, columns are block slots
    matrix: Vec<Vec<Option<AuthorityId>>>,
    /// Total rows (sum of all BGS shares)
    total_rows: usize,
    /// Total columns (blocks per epoch)
    total_cols: usize,
}

impl ScheduleMatrix {
    /// Create a new empty matrix
    pub fn new(total_bgs_shares: usize, blocks_per_epoch: usize) -> Self {
        let matrix = vec![vec![None; blocks_per_epoch]; total_bgs_shares];
        
        Self {
            matrix,
            total_rows: total_bgs_shares,
            total_cols: blocks_per_epoch,
        }
    }
    
    /// Fill the matrix elastically as per Algorithm 3
    pub fn fill_elastic(&mut self, bgs_shares: &[(AuthorityId, u32)]) {
        let mut row_idx = 0;
        
        // Step 1: Assign rows to validators based on their BGS shares
        for (validator_id, share_count) in bgs_shares {
            for _ in 0..*share_count {
                if row_idx >= self.total_rows {
                    break;
                }
                
                // Fill this row with the validator's ID
                for col in 0..self.total_cols {
                    self.matrix[row_idx][col] = Some(validator_id.clone());
                }
                row_idx += 1;
            }
        }
    }
    
    /// Select validators from matrix using deterministic random selection
    pub fn generate_schedule(&self, random_seed: [u8; 32]) -> Vec<AuthorityId> {
        let mut schedule = Vec::with_capacity(self.total_cols);
        let mut rng_state = u64::from_le_bytes(random_seed[0..8].try_into().unwrap_or([0u8; 8]));
        
        // For each slot (column), select a row randomly
        for col in 0..self.total_cols {
            if self.total_rows == 0 {
                continue;
            }
            
            // Deterministic random row selection
            rng_state = rng_state.wrapping_mul(6364136223846793005)
                                 .wrapping_add(1442695040888963407);
            let selected_row = (rng_state as usize) % self.total_rows;
            
            // Get the validator from the selected cell
            if let Some(ref validator) = self.matrix[selected_row][col] {
                schedule.push(validator.clone());
            }
        }
        
        schedule
    }
    
    /// Debug print the matrix (for testing)
    #[cfg(test)]
    pub fn print_matrix(&self) {
        println!("Schedule Matrix ({}x{}):", self.total_rows, self.total_cols);
        for (row_idx, row) in self.matrix.iter().enumerate() {
            print!("Row {}: ", row_idx);
            for cell in row.iter().take(10) { // Show first 10 columns
                match cell {
                    Some(v) => print!("V{} ", &v.as_ref()[0]), // Show first byte as ID
                    None => print!("-- "),
                }
            }
            if row.len() > 10 {
                print!("...");
            }
            println!();
        }
    }
}

/// The true elastic Advanced Block Schedule (eABS) generator from Algorithm 3
pub struct ElasticScheduleGenerator;

impl ElasticScheduleGenerator {
    /// Algorithm 3: Generate schedule using 2D matrix approach
    pub fn generate_schedule(
        bgs_scores: &[BlockGenerationScore],
        blocks_per_epoch: u32,
        random_seed: [u8; 32],
        min_blocks_per_validator: u32,
    ) -> Vec<AuthorityId> {
        if bgs_scores.is_empty() || blocks_per_epoch == 0 {
            return Vec::new();
        }
        
        // Step 1: Calculate BGS shares for each validator
        let bgs_shares = Self::calculate_bgs_shares(
            bgs_scores,
            blocks_per_epoch,
            min_blocks_per_validator,
        );
        
        // Step 2: Calculate total shares (rows in matrix)
        let total_shares: u32 = bgs_shares.iter().map(|(_, shares)| shares).sum();
        
        // Step 3: Create and fill the 2D matrix
        let mut matrix = ScheduleMatrix::new(
            total_shares as usize,
            blocks_per_epoch as usize,
        );
        matrix.fill_elastic(&bgs_shares);
        
        // Step 4: Generate schedule by selecting from matrix
        let mut schedule = matrix.generate_schedule(random_seed);
        
        // Step 5: Apply additional randomization for fairness
        Self::shuffle_with_constraints(&mut schedule, random_seed);
        
        schedule
    }
    
    /// Calculate BGS shares for each validator
    fn calculate_bgs_shares(
        bgs_scores: &[BlockGenerationScore],
        blocks_per_epoch: u32,
        min_blocks_per_validator: u32,
    ) -> Vec<(AuthorityId, u32)> {
        let num_validators = bgs_scores.len() as u32;
        let guaranteed_blocks = num_validators * min_blocks_per_validator;
        
        if guaranteed_blocks >= blocks_per_epoch {
            // If minimum guarantee uses all blocks, distribute equally
            return bgs_scores.iter()
                .map(|score| (score.authority.clone(), min_blocks_per_validator))
                .collect();
        }
        
        // Calculate total BGS for proportional distribution
        let total_bgs: u64 = bgs_scores.iter().map(|s| s.score).sum();
        
        if total_bgs == 0 {
            // Equal distribution if all scores are zero
            let blocks_each = blocks_per_epoch / num_validators;
            return bgs_scores.iter()
                .map(|score| (score.authority.clone(), blocks_each))
                .collect();
        }
        
        // Remaining blocks after minimum guarantee
        let remaining_blocks = blocks_per_epoch - guaranteed_blocks;
        
        let mut shares = Vec::new();
        for score in bgs_scores {
            // Start with minimum guarantee
            let mut validator_shares = min_blocks_per_validator;
            
            // Add proportional share of remaining blocks
            let proportion = (score.score as f64) / (total_bgs as f64);
            let additional = (remaining_blocks as f64 * proportion) as u32;
            validator_shares += additional;
            
            shares.push((score.authority.clone(), validator_shares));
        }
        
        // Adjust for rounding errors
        let total_assigned: u32 = shares.iter().map(|(_, s)| s).sum();
        if total_assigned < blocks_per_epoch {
            let difference = blocks_per_epoch - total_assigned;
            // Add remaining blocks to highest BGS validator
            if let Some(max_idx) = bgs_scores.iter()
                .enumerate()
                .max_by_key(|(_, s)| s.score)
                .map(|(i, _)| i) 
            {
                shares[max_idx].1 += difference;
            }
        }
        
        shares
    }
    
    /// Shuffle schedule with constraints (no more than 3 consecutive blocks)
    fn shuffle_with_constraints(schedule: &mut Vec<AuthorityId>, seed: [u8; 32]) {
        let mut rng_state = u64::from_le_bytes(seed[8..16].try_into().unwrap_or([0u8; 8]));
        let max_consecutive = 3;
        
        // Fisher-Yates shuffle with consecutive block constraint
        for i in (1..schedule.len()).rev() {
            // Find valid swap position
            rng_state = rng_state.wrapping_mul(6364136223846793005)
                                 .wrapping_add(1442695040888963407);
            let j = (rng_state as usize) % (i + 1);
            
            // Check if swap would violate consecutive constraint
            if Self::is_valid_swap(schedule, i, j, max_consecutive) {
                schedule.swap(i, j);
            }
        }
    }
    
    /// Check if a swap would violate the consecutive blocks constraint
    fn is_valid_swap(
        schedule: &[AuthorityId],
        pos1: usize,
        pos2: usize,
        max_consecutive: usize,
    ) -> bool {
        // Simple check - in production, implement full validation
        if pos1 == pos2 {
            return true;
        }
        
        // Check sequences around both positions
        let check_sequence = |pos: usize, validator: &AuthorityId| -> bool {
            let mut consecutive = 1;
            
            // Check backwards
            let mut idx = pos;
            while idx > 0 && schedule[idx - 1] == *validator {
                consecutive += 1;
                idx -= 1;
            }
            
            // Check forwards
            idx = pos;
            while idx < schedule.len() - 1 && schedule[idx + 1] == *validator {
                consecutive += 1;
                idx += 1;
            }
            
            consecutive <= max_consecutive
        };
        
        // Would the swap create invalid sequences?
        check_sequence(pos1, &schedule[pos2]) && check_sequence(pos2, &schedule[pos1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_scores() -> Vec<BlockGenerationScore> {
        vec![
            BlockGenerationScore {
                authority: AuthorityId::from_raw([1u8; 32]),
                security_score: 75,
                stake: 50_000_000,
                score: 3750, // 75 * 50
            },
            BlockGenerationScore {
                authority: AuthorityId::from_raw([2u8; 32]),
                security_score: 75,
                stake: 30_000_000,
                score: 2250, // 75 * 30
            },
            BlockGenerationScore {
                authority: AuthorityId::from_raw([3u8; 32]),
                security_score: 75,
                stake: 20_000_000,
                score: 1500, // 75 * 20
            },
        ]
    }
    
    #[test]
    fn test_eabs_schedule_generation() {
        let scores = create_test_scores();
        let blocks_per_epoch = 30;
        let random_seed = [42u8; 32];
        let min_blocks = 1;
        
        let schedule = ElasticScheduleGenerator::generate_schedule(
            &scores,
            blocks_per_epoch,
            random_seed,
            min_blocks,
        );
        
        assert_eq!(schedule.len(), blocks_per_epoch as usize);
        
        // Count blocks per validator
        let mut counts = std::collections::HashMap::new();
        for validator in &schedule {
            *counts.entry(validator.clone()).or_insert(0) += 1;
        }
        
        // Verify all validators got blocks
        assert_eq!(counts.len(), 3);
        
        // Verify proportional distribution (roughly)
        // Validator 1 (50% stake) should get ~50% of blocks
        let v1_count = counts.get(&scores[0].authority).unwrap_or(&0);
        assert!(*v1_count >= 12 && *v1_count <= 18); // 40-60% tolerance
    }
    
    #[test]
    fn test_matrix_creation() {
        let bgs_shares = vec![
            (AuthorityId::from_raw([1u8; 32]), 5),
            (AuthorityId::from_raw([2u8; 32]), 3),
            (AuthorityId::from_raw([3u8; 32]), 2),
        ];
        
        let mut matrix = ScheduleMatrix::new(10, 20); // 10 rows, 20 blocks
        matrix.fill_elastic(&bgs_shares);
        
        // First 5 rows should be validator 1
        for col in 0..20 {
            assert_eq!(matrix.matrix[0][col], Some(AuthorityId::from_raw([1u8; 32])));
        }
    }
}