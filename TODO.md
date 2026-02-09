# TODO

Add traversal functions for traversing a grid.
Add a check to make sure geometries are in the correct ranges?

Add a way to find the parent, top level HexCell?
Add something like this into the HexCell impl... There is no implicit, built in hierarchy in the n3gb index system. Each zoom is independent from the other.
```rust
impl HexCell {                                                                                           
     pub fn parent(&self, parent_zoom: u8) -> Result<HexCell, N3gbError> {                                
         // Will have to  recompute from same center at lower zoom                                                 
         HexCell::from_bng(&self.center, parent_zoom)                                                     
     }                                                                                                    
                                                                                                          
     pub fn children(&self, child_zoom: u8) -> Vec<HexCell> {                                             
         // Find all cells at child_zoom whose centers fall within self                                   
         // Not sure here yet                                                                         
     }                                                                                                    
 }    
```
