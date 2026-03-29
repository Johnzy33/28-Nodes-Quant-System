
use async_trait::async_trait; 
use crate::traits::{DataViewExt};
use shared_models::data_model::DataService;


pub fn generate_context<T, R, F>(data: &[T], mut transform: F) -> Vec<R>
where
    F: FnMut(&T, Option<&T>, Option<&T>) -> Option<R>,
{
    let mut results = Vec::new();
    
    // We use indices or windows to avoid cloning the elements inside the loop
    for i in 0..data.len() {
        let current = &data[i];
        let p1 = if i >= 1 { Some(&data[i - 1]) } else { None };
        let p2 = if i >= 2 { Some(&data[i - 2]) } else { None };

        if let Some(res) = transform(current, p1, p2) {
            results.push(res);
        }
    }
    results
}

#[async_trait]
impl DataViewExt for DataService {

}



