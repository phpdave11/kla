Kla intends to make interacting with HTTP APIs as easy as possible. There are multiple examples of documentation specifying shorthand http requests that look something like this:

> Create a notification:
> API `GET /api/v1/notify 'my notification message'`

Kla aims to make this a possibility by utilizing this exact structure, in fact putting `kla` in front of that in the terminal would run the http request. The following rules define how kla interprets your request.

With one argument Kla assumes a `GET` request, and no body:

```bash
kla /_cat/nodes
# is equivalent to
curl 'http://myenvironment.example.com/_cat/nodes'
```

With two arguments, Kla assumes the first is a method, and the second is the uri:

```bash
kla post /myindex/_rollover
# is equivalent to
curl -X POST 'http://myenvironment.example.com/myindex/_rollover'
```

Finally with three arguments, the assumption is method, uri and body:
```bash
kla post /myindex/_settings '{ "persistent" : { "cluster.routing.allocation.exclude._ip" : "10.0.0.1" } }'
# is equivalent to
curl -X POST 'http://myenvironment.example.com/myindex/_rollover' --data-binary '{ "persistent" : { "cluster.routing.allocation.exclude._ip" : "10.0.0.1" } }'
```

The body can also be preceded by an `@` symbol to denote a filepath, or a `-` to tell kla to read from standard in!

```bash
# Create a file with some body you want to send
echo '{
  "persistent" : {
    "cluster.routing.allocation.exclude._ip" : "10.0.0.1"
  }
}' > /tmp/my_settings.json

# specify the contents through stdin
cat /tmp/my_settings.json | kla post /myindex/_settings '-'

# specify the contents with a filepath
kla post /myindex/_settings '@'
```
