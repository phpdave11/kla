use tera::{Context, Tera};

// FetchMany allows us to collect multiple templates into RenderGroups
// which can be called later to render
pub trait FetchMany {
    fn has(&self, name: &str) -> bool;
    /// fetch_with_prefix will fetch a RenderGroup of the templates that have the prefixed name.
    fn fetch_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
        context: &'a Context,
    ) -> impl Iterator<Item = RenderGroup<'a>>;
}

impl FetchMany for Tera {
    fn has<'a>(&self, name: &str) -> bool {
        self.get_template_names()
            .filter(move |tmpl| *tmpl == name)
            .next()
            .is_some()
    }

    /// fetch_with_prefix will fetch a RenderGroup of the templates that have the prefixed name.
    fn fetch_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
        context: &'a Context,
    ) -> impl Iterator<Item = RenderGroup<'a>> {
        self.get_template_names()
            .filter(move |tmpl| tmpl.starts_with(prefix))
            .map(move |f| RenderGroup {
                name: f.strip_prefix(prefix).unwrap_or(f).into(),
                tmpl_name: f.into(),
                tmpl: self,
                context: context,
            })
    }
}

// TODO: This RenderGroup stuff isn't needed, we don't need to defer execution until later
/// A RenderGroup has all the context required to render a template held within
/// a Tera object.
pub struct RenderGroup<'a> {
    /// the basename of the template, this is the name used when turning this
    /// into a key value
    pub name: String,
    /// the name of the template in Tera
    pub tmpl_name: String,
    /// Tera, where this thing is held
    pub tmpl: &'a Tera,
    /// context is the context to render
    pub context: &'a Context,
}

impl<'a> RenderGroup<'a> {
    /// render will output the value of the evaluated template
    pub fn render(&self) -> std::result::Result<String, tera::Error> {
        self.tmpl.render(self.tmpl_name.as_str(), &self.context)
    }

    /// return the name of the template which will be rendered
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}
