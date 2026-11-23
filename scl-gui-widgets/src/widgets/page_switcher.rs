//! 页面栈和返回逻辑，对应旧组件库的 `PageSwitcher`。
//!
//! Iced 的页面切换由应用状态驱动，不再需要 Druid 那种保存子组件树的
//! `PageSwitcher` 组件。这里保留页面栈语义，方便把旧页面流程直接搬过来。

/// 一个简单的页面栈。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSwitcher<P> {
    chain: Vec<P>,
}

impl<P: Clone + PartialEq> PageSwitcher<P> {
    /// 创建一个以 `root` 为首页的页面栈。
    pub fn new(root: P) -> Self {
        Self { chain: vec![root] }
    }

    /// 当前页面。
    pub fn current(&self) -> &P {
        self.chain.last().expect("PageSwitcher 至少包含一个页面")
    }

    /// 当前页面栈。
    pub fn chain(&self) -> &[P] {
        &self.chain
    }

    /// 是否可以返回。
    pub fn can_go_back(&self) -> bool {
        self.chain.len() > 1
    }

    /// 推入一个新页面。
    pub fn push(&mut self, page: P) {
        if self.current() != &page {
            self.chain.push(page);
        }
    }

    /// 返回上一页；如果已经在首页则返回 `None`。
    pub fn pop(&mut self) -> Option<P> {
        if self.chain.len() > 1 {
            self.chain.pop()
        } else {
            None
        }
    }

    /// 一直返回到指定页面；如果页面不在栈中则不做任何事。
    pub fn pop_to(&mut self, page: &P) -> bool {
        if let Some(index) = self.chain.iter().position(|item| item == page) {
            self.chain.truncate(index + 1);
            true
        } else {
            false
        }
    }

    /// 丢弃当前栈并重置到指定页面。
    pub fn reset_to(&mut self, page: P) {
        self.chain.clear();
        self.chain.push(page);
    }
}

#[cfg(test)]
mod tests {
    use super::PageSwitcher;

    #[test]
    fn page_stack_works() {
        let mut pages = PageSwitcher::new("main");
        assert_eq!(*pages.current(), "main");
        assert!(!pages.can_go_back());

        pages.push("settings");
        pages.push("settings");
        assert_eq!(pages.chain(), &["main", "settings"]);
        assert!(pages.can_go_back());

        pages.push("about");
        assert!(pages.pop_to(&"settings"));
        assert_eq!(*pages.current(), "settings");
        assert_eq!(pages.pop(), Some("settings"));
        assert_eq!(*pages.current(), "main");
        assert_eq!(pages.pop(), None);

        pages.push("about");
        pages.reset_to("main");
        assert_eq!(pages.chain(), &["main"]);
    }
}
