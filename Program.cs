using Microsoft.VisualBasic.FileIO;
using static Utils;

class Lox
{
    static bool had_error = false;
    static bool had_rt_error = false;
    static Interpreter interpreter = new();

    static void Main(string[] args)
    {
        switch (args.Length)
        {
            case 0:
                RunPrompt();
                break;
            case 1:
                RunFile(args[0]);
                break;
            default:
                Console.WriteLine("Requires 0 or 1 args");
                break;
        }
    }

    static void RunFile(String path)
    {
        string stream = File.ReadAllText(path);
        Run(stream);
        if (had_error) Environment.Exit(65);
        if (had_rt_error) Environment.Exit(70);
    }

    static void RunPrompt()
    {
        while (true)
        {
            Console.Write("> ");
            var line = Console.ReadLine();
            if (line == "") break;
            Run(line!);
            had_error = false;
        }
    }

    static void Run(string code)
    {
        var tokens = new Scanner(code).ScanTokens();
        Parser parser = new(tokens);
        List<Stmt.Any> stmts = parser.Parse();

        if (had_error) return;

        Resolver resolver = new(interpreter);
        resolver.Resolve(stmts);

        if (had_error) return;

        interpreter.Interpret(stmts);
        Console.WriteLine($"Execution complete in {FormatTime(interpreter.clock.ElapsedTicks)}");
    }

    public static void Error(int line, string msg)
    {
        Report(line, "", msg);
    }

    public static void Error(Token t, string msg)
    {
        if (t.type == TokenType.EOF)
        {
            Report(t.line, "at end", msg);
        }
        else
        {
            Report(t.line, "at '" + t.token + "'", msg);
        }
    }

    static void Report(int line, string loc, string msg)
    {
        Console.WriteLine($"[line {line}] Error {loc}: {msg}");
        had_error = true;
    }

    public static void RuntimeError(RuntimeError e)
    {
        Console.Error.WriteLine("[line " + e.token.line + "] Error: " + e.Message);
        had_rt_error = true;
    }
}