> Usage: kla [OPTIONS] [METHOD_OR_URL] [URL] [BODY] [COMMAND]
> 
> Commands:
>   environments  Show the environments that are available to you.
>   switch        Select an environment to be the current context
>   run           run templates defined for the environment
>   help          Print this message or the help of the given subcommand(s)
> 
> Arguments:
>   [METHOD_OR_URL]  The URL path (with an assumed GET method) OR the method if another argument is supplied
>   [URL]            The URL path when a method is supplied
>   [BODY]           The body of the HTTP request, if prefixed with a `@` it is treated as a file path
> 
> Options:
>       --agent <AGENT>
>           The header agent string [default: kla]
>   -e, --env <ENVIRONMENT>
>           The environment we will run the request against [default: poetry]
>   -t, --template <TEMPLATE>
>           The template to use when formating the output. prepending with @ will read a file.
>       --failure-template <TEMPLATE>
>           The template to use when formating the failure output. prepending with @ will read a file.
>   -o, --output <FILE>
>           The file to write the output into
>       --timeout <SECONDS>
>           The amount of time allotted for the request to finish
>       --basic-auth <BASIC_AUTH>
>           The username and password seperated by :, a preceding @ denotes a file path.
>       --bearer-token <BEARER_TOKEN>
>           The bearer token to use in requests. A preceding @ denotes a file path.
>   -H, --header <HEADER>
>           Specify a header The key and value should be seperated by a : (eg --header "Content-Type: application/json")
>   -Q, --query <QUERY>
>           Specify a query parameter The key and value should be seperated by a = (eg --query "username=Jed")
>   -F, --form <FORM>
>           Specify a form key=value to be passed in the form body
>   -v, --verbose
>           make it loud and proud
>       --dry
>           don't actually do anything, will automatically enable verbose
>       --http-version <HTTP_VERSION>
>           The version of http to send the request as [possible values: 0.9, 1.0, 1.1, 2.0, 3.0]
>       --no-gzip
>           Do not automatically uncompress gzip responses
>       --no-brotli
>           Do not automatically uncompress brotli responses
>       --no-deflate
>           Do not automatically uncompress deflate responses
>       --max-redirects <NUMBER>
>           The number of redirects allowed
>       --no-redirects
>           Disable any redirects
>       --proxy <PROXY>
>           The proxy to use for all requests.
>       --proxy-http <PROXY_HTTP>
>           The proxy to use for http requests.
>       --proxy-https <PROXY_HTTPS>
>           The proxy to use for https requests.
>       --proxy-auth <PROXY_AUTH>
>           The username and password seperated by :.
>       --connect-timeout <DURATION>
>           The amount of time to allow for connection
>       --sigv4
>           Sign the request with AWS v4 Signature
>       --sigv4-aws-profile <AWS_PROFILE>
>           The AWS profile to use when signing a request
>       --sigv4-service <SERVICE>
>           The AWS Service to use when signing the request
>       --certificate <CERTIFICATE_FILE>
>           The path to the certificate to use for requests. Accepts PEM and DER, expects files to end in .der or .pem. defaults to pem
>   -h, --help
>           Print help
>   -V, --version
>           Print version
