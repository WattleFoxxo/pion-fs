use super::{FileSystem, Error};

pub struct Path<'a> {
    path: &'a str,
    name_index: usize,
    path_index: usize,
}

impl <'a> Path<'a> {
    pub fn new<F: FileSystem>(path: &'a str) -> Result<Self, Error> {
        let mut name_index = 0;

        for (i, c) in path.chars().enumerate() {
            if c == '\\' || c == '/' {
                name_index = i + 1;
            }
        }

        Ok(Self {
            path,
            name_index,
            path_index: 0,
        })
    }

    pub fn name(&self) -> &str {
        &self.path[self.name_index..]
    }
}

impl <'a> Iterator for Path<'a> 
{
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        for (i, c) in (&self.path[self.path_index..self.name_index]).chars().enumerate() {
            if c == '\\' || c == '/' {
                let start = self.path_index;
                let end = start + i;
                self.path_index += i + 1;
                return Some(&self.path[start..end]);
                
            }
        }

        None
    }
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    #[test]
//    fn it_works() {
//        let mut path = Path::new("/folder/file_name.txt").unwrap();
//        println!("name: {}", path.name());
//        for p in path {
//            println!("dir: {}", p.unwrap());
//        }
//        let result = 2 + 2;
//        assert_eq!(result, 4);
//    }
//}