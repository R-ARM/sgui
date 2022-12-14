#[derive(Debug)]
pub struct Layout {
    tabs: Vec<Tab>,
}

impl Layout {
    pub fn tab_names(&self) -> Vec<&str> {
        self.tabs.iter()
            .map(|v| v.name())
            .collect()
    }
    pub fn tab_count(&self) -> i32 {
        self.tabs.len() as i32 - 1
    }
    pub fn tab(&self, number: usize) -> Option<&Tab> {
        self.tabs.get(number)
    }
    pub fn tab_mut(&mut self, number: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(number)
    }
    pub fn builder() -> LayoutBuilder {
        LayoutBuilder::new()
    }
}
#[derive(Debug)]
pub struct Tab {
    name: String,
    item_grid: Vec<Vec<Item>>,
}

impl Tab {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn items(&self) -> &Vec<Vec<Item>> {
        &self.item_grid
    }
    pub fn items_mut(&mut self) -> &mut Vec<Vec<Item>> {
        &mut self.item_grid
    }
}

#[derive(Debug)]
pub enum Item {
    Text(String),
    StatefulButton(String, bool, u128),
    StatelessButton(String, u128),
}

pub struct LayoutBuilder {
    tabs: Vec<Tab>,
}

impl LayoutBuilder {
    pub fn new() -> LayoutBuilder {
        LayoutBuilder {
            tabs: Vec::new(),
        }
    }
    pub fn tab(self, name: &str) -> TabBuilder {
        TabBuilder {
            layout_builder: Some(self),
            lines: Vec::new(),
            name: name.to_string(),
        }
    }
}

pub struct TabBuilder {
    lines: Vec<Vec<Item>>,
    name: String,
    layout_builder: Option<LayoutBuilder>,
}

impl TabBuilder {
    pub fn line(self) -> LineBuilder {
        LineBuilder {
            tab_builder: Some(self),
            items: Vec::new(),
        }
    }
    pub fn build(mut self) -> Layout {
        let mut layout_builder = self.layout_builder.take().unwrap();
        layout_builder.tabs.push(Tab{ item_grid: self.lines, name: self.name });

        Layout {
            tabs: layout_builder.tabs,
        }
    }
    pub fn tab(mut self, name: &str) -> TabBuilder {
        let mut layout_builder = self.layout_builder.take().unwrap();
        layout_builder.tabs.push(Tab{ item_grid: self.lines, name: self.name });

        layout_builder.tab(name)
    }
}

pub struct LineBuilder {
    items: Vec<Item>,
    tab_builder: Option<TabBuilder>,
}

impl LineBuilder {
    pub fn text(mut self, text: &str) -> LineBuilder {
        self.items.push(Item::Text(text.to_string()));
        self
    }
    pub fn button_stateful(mut self, text: &str, init_state: bool, id: u128) -> LineBuilder {
        self.items.push(Item::StatefulButton(text.to_string(), init_state, id));
        self
    }
    pub fn button_stateless(mut self, text: &str, id: u128) -> LineBuilder {
        self.items.push(Item::StatelessButton(text.to_string(), id));
        self
    }
    pub fn line(mut self) -> LineBuilder {
        let mut tab_builder = self.tab_builder.take().unwrap();
        tab_builder.lines.push(self.items);

        LineBuilder {
            tab_builder: Some(tab_builder),
            items: Vec::new(),
        }
    }
    pub fn tab(mut self, name: &str) -> TabBuilder {
        let mut tab_builder = self.tab_builder.take().unwrap();
        tab_builder.lines.push(self.items);

        tab_builder.tab(name)
    }
    pub fn endl(mut self) -> TabBuilder {
        let mut tab_builder = self.tab_builder.take().unwrap();
        tab_builder.lines.push(self.items);
        tab_builder
    }

    pub fn build(mut self) -> Layout {
        let mut tab_builder = self.tab_builder.take().unwrap();
        tab_builder.lines.push(self.items);

        tab_builder.build()
    }
}
