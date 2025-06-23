//! 路径(Path)模块
//!
//! 提供路径字符串的解析、拼接、父目录等操作。

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// 路径结构体，支持分层管理和操作。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    inner: Vec<String>,
}

impl Path {
    /// 由字符串创建路径。
    pub fn new(path: String) -> Self {
        let parts: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s: &str| s.to_string())
            .collect();
        Self { inner: parts }
    }

    /// 获取路径各层级的字符串数组。
    pub fn get_inner(&self) -> Vec<String> {
        self.inner.clone()
    }

    /// 获取路径最后一级名称。
    pub fn get_name(&self) -> String {
        if self.inner.is_empty() {
            "".to_string()
        } else {
            self.inner.last().unwrap().to_string()
        }
    }

    /// 获取父目录路径字符串。
    pub fn parent(&self) -> String {
        let mut inner = self.inner.clone();
        inner.pop();
        Path { inner }.to_string()
    }
    
    /// 转为字符串（绝对路径）。
    pub fn to_string(&self) -> String {
        if self.inner.is_empty() {
            return "/".to_string();
        }
        let mut s = String::from("/");
        s.push_str(&self.inner.join("/"));
        s
    }

    /// 拼接子路径。
    pub fn join(&self, path: &str) -> Path {
        let mut inner = self.inner.clone();
        inner.push(path.to_string());
        Path { inner }
    }

    /// 是否为当前目录。
    pub fn is_current(&self) -> bool {
        self.inner.len() == 1 && self.inner[0] == "."
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Path::new(s.to_string())
    }
}

impl From<String> for Path {
    fn from(s: String) -> Self {
        Path::new(s)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_path_new_simple_cases() {
        // Test with simple path
        let path = Path::new("foo/bar".to_string());
        assert_eq!(path.get_inner(), vec!["foo".to_string(), "bar".to_string()]);

        // Test with leading slash
        let path = Path::new("/foo/bar".to_string());
        assert_eq!(path.get_inner(), vec!["foo".to_string(), "bar".to_string()]);

        // Test with trailing slash
        let path = Path::new("foo/bar/".to_string());
        assert_eq!(path.get_inner(), vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn test_path_new_complex_cases() {
        // Test with multiple slashes
        let path = Path::new("foo//bar".to_string());
        assert_eq!(path.get_inner(), vec!["foo".to_string(), "bar".to_string()]);

        // Test with many path components
        let path = Path::new("/usr/local/bin/echo".to_string());
        assert_eq!(
            path.get_inner(),
            vec![
                "usr".to_string(),
                "local".to_string(),
                "bin".to_string(),
                "echo".to_string()
            ]
        );

        // Test with mixed consecutive slashes
        let path = Path::new("/usr///local/bin//echo///".to_string());
        assert_eq!(
            path.get_inner(),
            vec![
                "usr".to_string(),
                "local".to_string(),
                "bin".to_string(),
                "echo".to_string()
            ]
        );

        // Test with dots
        let path = Path::new("./foo/./bar".to_string());
        assert_eq!(
            path.get_inner(),
            vec![
                ".".to_string(),
                "foo".to_string(),
                ".".to_string(),
                "bar".to_string()
            ]
        );

        // Test with special characters
        let path = Path::new("foo$bar/baz#qux".to_string());
        assert_eq!(
            path.get_inner(),
            vec!["foo$bar".to_string(), "baz#qux".to_string()]
        );
    }

    #[test]
    fn test_path_new_edge_cases() {
        // Test with empty path
        let path = Path::new("".to_string());
        assert_eq!(path.get_inner(), Vec::<String>::new());

        // Test with just slashes
        let path = Path::new("/".to_string());
        assert_eq!(path.get_inner(), Vec::<String>::new());

        // Test with multiple slashes only
        let path = Path::new("///".to_string());
        assert_eq!(path.get_inner(), Vec::<String>::new());

        // Test with very long path (testing performance/memory)
        let long_path = "/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z".to_string();
        let path = Path::new(long_path);
        assert_eq!(path.get_inner().len(), 26); // a through z = 26 components
    }

    #[test]
    fn test_get_name_normal_cases() {
        // Test with normal path
        let path = Path::new("foo/bar".to_string());
        assert_eq!(path.get_name(), "bar".to_string());

        // Test with single component
        let path = Path::new("foo".to_string());
        assert_eq!(path.get_name(), "foo".to_string());

        // Test with long path, multiple components
        let path = Path::new("/usr/local/bin/echo".to_string());
        assert_eq!(path.get_name(), "echo".to_string());
    }

    #[test]
    fn test_get_name_edge_cases() {
        // Test with empty path
        let path = Path::new("".to_string());
        assert_eq!(path.get_name(), "".to_string());

        // Test with only slashes
        let path = Path::new("/////".to_string());
        assert_eq!(path.get_name(), "".to_string());

        // Test with trailing slashes
        let path = Path::new("foo/bar///".to_string());
        assert_eq!(path.get_name(), "bar".to_string());
    }

    #[test]
    fn test_clone_behavior() {
        // Verify that get_inner() returns a clone and doesn't modify original
        let path = Path::new("foo/bar".to_string());
        let inner1 = path.get_inner();
        let inner2 = path.get_inner();

        // Both should be equal but separate instances
        assert_eq!(inner1, inner2);
        assert_eq!(inner1, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn test_with_unicode_paths() {
        // Test with Unicode characters in path
        let path = Path::new("🦀/rust/专用".to_string());
        assert_eq!(
            path.get_inner(),
            vec!["🦀".to_string(), "rust".to_string(), "专用".to_string()]
        );
        assert_eq!(path.get_name(), "专用".to_string());
    }
}
