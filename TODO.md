# TODO

Add traversal functions for traversing a grid at a certain zoom
  e.g. How far away is the thing in this hexcell from this other thing in that other hexcell?

Add a check to make sure geometries are always in the correct range/bounds.

Add a way to reject non valid geometries at the start.

Add a way to find the parent, top level HexCell. N3gb is not internally hierarchical like H3 - you'd need to recompute the parent each time - need to think about that.

Add something like this into the HexCell impl... There is no implicit hierarchy in the n3gb index system. Each zoom is independent from the other.

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
