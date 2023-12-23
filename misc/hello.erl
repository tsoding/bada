-module(hello).
-export([hello/0, sum/2]).

hello() ->
    io:format("Hello, World\n").

sum(X, Y) ->
    X + Y.
