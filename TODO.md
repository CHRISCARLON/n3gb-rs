# TODO

Add traversal functions for traversing a grid at a certain zoom

Add a check to make sure geometries are in the correct ranges?

Add a way to find the parent, top level HexCell? N3gb is not internally hierarchical like H3 - you'd need to recompute the parent each time - need to think about that.

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

Maybe the hex cell from geometry should be the front door for the transformations?

```rust
HexCell::from_geometry
```


There's probably 2 main paths to consider for the IO stuff...

- When you want to output one row, per thing, with it's hex id
- When you want to output one row, per hex id, with an aggregated count of things

^^^ Different use cases, 2nd one pre-computes the work required for hex density maps etc
Maybe we only focus on the 1st for this lib for now or we create a seperate density module later
