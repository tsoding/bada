-module(beam).
-export([read/1]).

read(Filename) ->
    {ok, File} = file:read_file(Filename),
    <<"FOR1",
      Size:32/integer,
      "BEAM",
      Chunks/binary>> = File,
    {Size, parse_chunks(read_chunks(Chunks, []), [])}.

read_chunks(<<N,A,M,E,Size:32/integer,Tail/binary>>, Acc) ->
    ChunkLength = align_by_four(Size),
    <<Chunk:ChunkLength/binary, Rest/binary>> = Tail,
    read_chunks(Rest, [{[N,A,M,E], Size, Chunk}|Acc]);
read_chunks(<<>>, Acc) ->
    lists:reverse(Acc).

%% ImportChunk = <<
%%   ChunkName:4/unit:8 = "Code",
%%   ChunkSize:32/big,
%%   SubSize:32/big,
%%   InstructionSet:32/big,        % Must match code version in the emulator
%%   OpcodeMax:32/big,
%%   LabelCount:32/big,
%%   FunctionCount:32/big,
%%   Code:(ChunkSize-SubSize)/binary,  % all remaining data
%%   Padding4:0..3/unit:8
%% >>
parse_chunks([{"Code", Size, <<SubSize:32/big,
                               _Info:SubSize/binary,
                               Code/binary>>
              } | Rest], Acc) ->
    OpcodeSize = Size - (SubSize + 8),
    <<OpCodes:OpcodeSize/binary, _Align/binary>> = Code,
    parse_chunks(Rest, [{opcodes, OpCodes} | Acc]);
parse_chunks([Chunk | Rest], Acc) ->
    parse_chunks(Rest, [Chunk | Acc]);
parse_chunks([], Acc) ->
    Acc.

align_by_four(N) -> (4 * ((N+4-1) div 4)).
