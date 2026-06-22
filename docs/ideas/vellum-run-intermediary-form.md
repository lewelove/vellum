# vellum run intermediary form

the json passed to scripts stdin must contain an array of albums in order returned by sql + config definition

```json
{
  [
    {
      "album": {
        // ...
      }
    },
    {
      // etc...
    }
  ],

  "config": {}
}
```
