use kedo_core::{downcast_state, ClassTable, ProtoTable};
use kedo_utils::{downcast_ref, drop_ptr, map_err_from_option};
use rust_jsc::{
    callback, class::ClassError, constructor, finalize, has_instance, JSClass,
    JSClassAttribute, JSContext, JSError, JSFunction, JSObject, JSResult, JSValue,
    PrivateData, PropertyDescriptorBuilder,
};
use url::{ParseError, Url};

pub fn parse_url(ctx: &JSContext, args: &[JSValue]) -> JSResult<Url> {
    let url = match args.get(0) {
        Some(url) => url.as_string()?.to_string(),
        None => {
            return Err(JSError::new_typ(&ctx, "Expected 1 argument")?);
        }
    };

    let mut base_url: Option<Url> = None;
    let base_url_arg = match args.get(1) {
        Some(base_url) => {
            if base_url.is_null() || base_url.is_undefined() {
                None
            } else {
                Some(base_url.as_string()?.to_string())
            }
        }
        None => None,
    };

    if let Some(base_url_arg) = base_url_arg {
        base_url = match Url::parse(&base_url_arg.as_str()) {
            Ok(url) => Some(url),
            Err(e) => return Err(JSError::new_typ(&ctx, e.to_string())?),
        }
    }

    match Url::options().base_url(base_url.as_ref()).parse(&url) {
        Ok(url) => return Ok(url),
        Err(e) => return Err(JSError::new_typ(&ctx, e.to_string())?),
    };
}

pub struct ParsedUrl(Url);

impl ParsedUrl {
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    pub fn set_scheme(&mut self, scheme: &str) {
        self.0.set_scheme(scheme).unwrap();
    }

    pub fn username(&self) -> &str {
        self.0.username()
    }

    pub fn set_username(&mut self, username: &str) -> Result<(), ()> {
        self.0.set_username(username)
    }

    pub fn password(&self) -> Option<&str> {
        self.0.password()
    }

    pub fn set_password(&mut self, password: Option<&str>) -> Result<(), ()> {
        self.0.set_password(password)
    }

    pub fn host_str(&self) -> Option<&str> {
        self.0.host_str()
    }

    pub fn set_host_str(&mut self, host: Option<&str>) -> Result<(), ParseError> {
        self.0.set_host(host)
    }

    pub fn port(&self) -> Option<u16> {
        self.0.port()
    }

    pub fn set_port(&mut self, port: Option<u16>) -> Result<(), ()> {
        self.0.set_port(port)
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    pub fn set_path(&mut self, path: &str) {
        self.0.set_path(path);
    }

    pub fn query(&self) -> Option<&str> {
        self.0.query()
    }

    pub fn set_query(&mut self, query: Option<&str>) {
        self.0.set_query(query);
    }

    pub fn fragment(&self) -> Option<&str> {
        self.0.fragment()
    }

    pub fn set_fragment(&mut self, fragment: Option<&str>) {
        self.0.set_fragment(fragment);
    }

    pub fn origin(&self) -> String {
        self.0.origin().unicode_serialization()
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

pub struct UrlRecord {}

impl UrlRecord {
    pub const CLASS_NAME: &'static str = "UrlRecord";
    pub const PROTO_NAME: &'static str = "UrlRecordPrototype";

    pub fn init_proto(
        proto_manager: &mut ProtoTable,
        manager: &mut ClassTable,
        ctx: &JSContext,
    ) -> Result<(), ClassError> {
        let class = manager
            .get(UrlRecord::CLASS_NAME)
            .expect("UrlRecord class not found");

        let template_object = class.object::<ParsedUrl>(ctx, None);
        Self::set_properties(ctx, &template_object)
            .map_err(|_| ClassError::CreateFailed)?;
        proto_manager.insert(UrlRecord::PROTO_NAME.to_string(), template_object);
        Ok(())
    }

    pub fn template_object(ctx: &JSContext, scope: &JSObject) -> JSResult<()> {
        let state = downcast_state(ctx);
        let template_object = state
            .protos()
            .get(UrlRecord::PROTO_NAME)
            .expect("UrlRecordPrototype not found");

        scope.set_property(
            UrlRecord::CLASS_NAME,
            &template_object,
            Default::default(),
        )?;
        Ok(())
    }

    pub fn init_class(manaager: &mut ClassTable) -> Result<(), ClassError> {
        let builder = JSClass::builder(Self::CLASS_NAME);
        let class = builder
            .call_as_constructor(Some(Self::constructor))
            .set_finalize(Some(Self::finalize))
            .has_instance(Some(Self::has_instance))
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build()?;

        manaager.insert(class);
        Ok(())
    }

    pub fn is(ctx: &JSContext, object: &JSObject) -> JSResult<bool> {
        let state = downcast_state(ctx);
        let class = state.classes().get(Self::CLASS_NAME).unwrap();
        object.is_object_of_class(class)
    }

    /// finalize is called when the object is being garbage collected.
    /// This is the place to clean up any resources that the object may hold.
    #[finalize]
    fn finalize(data_ptr: PrivateData) {
        drop_ptr::<ParsedUrl>(data_ptr);
    }

    #[has_instance]
    fn has_instance(
        ctx: JSContext,
        _constructor: JSObject,
        value: JSValue,
    ) -> JSResult<bool> {
        let object = value.as_object()?;
        UrlRecord::is(&ctx, &object)
    }

    fn set_properties(ctx: &JSContext, this: &JSObject) -> JSResult<()> {
        let descriptor = PropertyDescriptorBuilder::new()
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build();

        let function = JSFunction::callback(&ctx, Some("get"), Some(Self::get));
        this.set_property("get", &function, descriptor)?;

        let function = JSFunction::callback(&ctx, Some("set"), Some(Self::set));
        this.set_property("set", &function, descriptor)?;

        let function =
            JSFunction::callback(&ctx, Some("toString"), Some(Self::to_string));
        this.set_property("toString", &function, descriptor)?;

        Ok(())
    }

    #[callback]
    fn get(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let parsed_url = downcast_ref::<ParsedUrl>(&this).unwrap();
        let key = match args.get(0) {
            Some(value) => value.as_string()?.to_string(),
            None => return Err(JSError::new_typ(&ctx, "Invalid argument")?),
        };

        let value = match key.as_str() {
            "scheme" => parsed_url.scheme(),
            "username" => parsed_url.username(),
            "origin" => return Ok(JSValue::string(&ctx, parsed_url.origin())),
            "password" => parsed_url.password().unwrap_or(""),
            "host" => match parsed_url.host_str() {
                Some(host) => host,
                None => return Ok(JSValue::null(&ctx)),
            },
            "port" => match parsed_url.port().map(|x| x.to_string()) {
                Some(port) => return Ok(JSValue::string(&ctx, port)),
                None => return Ok(JSValue::null(&ctx)),
            },
            "path" => parsed_url.path(),
            "query" => match parsed_url.query() {
                Some(query) => query,
                None => return Ok(JSValue::null(&ctx)),
            },
            "fragment" => match parsed_url.fragment() {
                Some(fragment) => fragment,
                None => return Ok(JSValue::null(&ctx)),
            },
            _ => return Err(JSError::new_typ(&ctx, "Invalid argument")?),
        };

        Ok(JSValue::string(&ctx, value))
    }

    #[callback]
    fn set(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut parsed_url = downcast_ref::<ParsedUrl>(&this).unwrap();
        let key = match args.get(0) {
            Some(value) => value.as_string()?.to_string(),
            None => return Err(JSError::new_typ(&ctx, "Invalid argument")?),
        };

        let value: Option<String> = match args.get(1) {
            Some(value) => value
                .is_string()
                .then(|| value.as_string().unwrap().to_string()),
            None => return Err(JSError::new_typ(&ctx, "Invalid argument")?),
        };

        match key.as_str() {
            "scheme" => parsed_url.set_scheme(map_err_from_option(&ctx, value)?.as_str()),
            "username" => parsed_url
                .set_username(map_err_from_option(&ctx, value)?.as_str())
                .map_err(|_| JSError::new_typ(&ctx, "Invalid argument").unwrap())?,
            "password" => parsed_url
                .set_password(value.as_deref())
                .map_err(|_| JSError::new_typ(&ctx, "Invalid argument").unwrap())?,
            "host" => parsed_url
                .set_host_str(value.as_deref())
                .map_err(|_| JSError::new_typ(&ctx, "Invalid argument").unwrap())?,
            "path" => parsed_url.set_path(map_err_from_option(&ctx, value)?.as_str()),
            "port" => {
                // let port_value = UrlRecord::set_port(&ctx, value)?;
                match value {
                    Some(value) => {
                        let port = value.parse::<u16>().map_err(|_| {
                            JSError::new_typ(&ctx, "Invalid argument").unwrap()
                        })?;

                        parsed_url.set_port(Some(port)).unwrap();
                    }
                    None => {
                        parsed_url.set_port(None).unwrap();
                    }
                }
            }
            "query" => parsed_url.set_query(value.as_deref()),
            "fragment" => parsed_url.set_fragment(value.as_deref()),
            _ => return Err(JSError::new_typ(&ctx, "Invalid argument")?),
        };

        Ok(JSValue::undefined(&ctx))
    }

    #[callback]
    fn to_string(
        ctx: JSContext,
        _: JSObject,
        this: JSObject,
        _: &[JSValue],
    ) -> JSResult<JSValue> {
        let parsed_url = downcast_ref::<ParsedUrl>(&this)
            .ok_or_else(|| JSError::new_typ(&ctx, "Invalid object").unwrap())?;
        Ok(JSValue::string(&ctx, parsed_url.to_string()))
    }

    #[constructor]
    fn constructor(
        ctx: JSContext,
        constructor: JSObject,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let url: Url = parse_url(&ctx, args)?;
        let state = downcast_state(&ctx);
        let class = state.classes().get(UrlRecord::CLASS_NAME).unwrap();
        let object = class.object::<ParsedUrl>(&ctx, Some(Box::new(ParsedUrl(url))));
        object.set_prototype(&constructor);
        Ok(object.into())
    }
}
