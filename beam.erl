-module(beam).
-export([read/1]).

-define(IGNORE_UNKNOWN_CHUNKS, true).

read(Filename) ->
    {ok, File} = file:read_file(Filename),
    <<"FOR1",
      Size:32/integer,
      "BEAM",
      Chunks/binary>> = File,
    {Size, parse_chunks(read_chunks(Chunks))}.

read_chunks(Chunks) ->
    lists:reverse(read_chunks(Chunks, [])).
read_chunks(<<N,A,M,E,Size:32/integer,Tail/binary>>, Acc) ->
    ChunkLength = align_by_four(Size),
    <<Chunk:ChunkLength/binary, Rest/binary>> = Tail,
    read_chunks(Rest, [{[N,A,M,E], Size, Chunk}|Acc]);
read_chunks(<<>>, Acc) ->
    Acc.

parse_chunks(Chunks) ->
    lists:reverse(parse_chunks(Chunks, [])).
%% AtomChunk = <<
%%   ChunkName:4/unit:8 = "Atom",
%%   ChunkSize:32/big,
%%   NumberOfAtoms:32/big,
%%   [<<AtomLength:8, AtomName:AtomLength/unit:8>> || repeat NumberOfAtoms],
%%   Padding4:0..3/unit:8
%% >>
parse_chunks([{"AtU8", _Size, <<_NumberOfAtoms:32/big, Atoms/binary>>} | Rest], Acc) ->
    parse_chunks(Rest, [{atoms, parse_atoms(Atoms)} | Acc]);
%% CodeChunk = <<
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
                               InstructionSet:32/big,
                               OpcodeMax:32/big,
                               LabelCount:32/big,
                               FunctionCount:32/big,
                               %% Info:SubSize/binary,
                               Code/binary>>
              } | Rest], Acc) ->
    OpcodeSize = Size - (SubSize + 8),
    <<OpCodes:OpcodeSize/binary, _Align/binary>> = Code,
    Info = [{sub_size, SubSize},
            {instruction_set, InstructionSet},
            {opcode_max, OpcodeMax},
            {label_count, LabelCount},
            {function_count, FunctionCount}],
    parse_chunks(Rest, [{code, Info, OpCodes} | Acc]);
%% ExportChunk = <<
%%   ChunkName:4/unit:8 = "ExpT",
%%   ChunkSize:32/big,
%%   ExportCount:32/big,
%%   [ << FunctionName:32/big,
%%        Arity:32/big,
%%        Label:32/big
%%     >> || repeat ExportCount ],
%%   Padding4:0..3/unit:8
%% >>
parse_chunks([{"ExpT", _Size, <<_ExportCount:32/big, Exports/binary>>} | Rest], Acc) ->
    parse_chunks(Rest, [{exports, parse_exports(Exports)} | Acc]);
%% ImportChunk = <<
%%   ChunkName:4/unit:8 = "ImpT",
%%   ChunkSize:32/big,
%%   ImportCount:32/big,
%%   [ << ModuleName:32/big,
%%        FunctionName:32/big,
%%        Arity:32/big
%%     >> || repeat ImportCount ],
%%   Padding4:0..3/unit:8
%% >>
parse_chunks([{"ImpT", _Size, <<_ImportCount:32/big, Imports/binary>>} | Rest], Acc) ->
    parse_chunks(Rest, [{imports, parse_imports(Imports)} | Acc]);
%% StringChunk = <<
%%   ChunkName:4/unit:8 = "StrT",
%%   ChunkSize:32/big,
%%   Data:ChunkSize/binary,
%%   Padding4:0..3/unit:8
%% >>
parse_chunks([{"StrT", _Size, <<Strings/binary>>} | Rest], Acc) ->
    parse_chunks(Rest, [{strings, binary_to_list(Strings)} | Acc]);
parse_chunks([Chunk | Rest], Acc) ->
    case ?IGNORE_UNKNOWN_CHUNKS of
       true -> parse_chunks(Rest, Acc);
       false -> parse_chunks(Rest, [Chunk | Acc])
    end;
parse_chunks([], Acc) ->
    Acc.

parse_atoms(<<AtomLength:8, AtomName:AtomLength/binary, Tail/binary>>) ->
    [binary_to_list(AtomName) | parse_atoms(Tail)];
parse_atoms(_Padding) -> [].

parse_exports(<<FunctionName:32/big, Arity:32/big, Label:32/big, Tail/binary>>) ->
    [[{function_name, FunctionName},
      {arity, Arity},
      {label, Label}] | parse_exports(Tail)];
parse_exports(_Padding) -> [].

parse_imports(<<ModuleName:32/big, FunctionName:32/big, Arity:32/big, Tail/binary>>) ->
    [[{module_name, ModuleName},
      {function_name, FunctionName},
      {arity, Arity}] | parse_imports(Tail)];
parse_imports(_Padding) -> [].


align_by_four(N) -> (4 * ((N+4-1) div 4)).
