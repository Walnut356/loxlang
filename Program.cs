using Microsoft.VisualBasic.FileIO;
using static Utils;

class Lox
{
    static bool had_error = false;

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
        if (had_error) throw new MalformedLineException();
    }

    static void RunPrompt()
    {
        while (true)
        {
            Console.Write("> ");
            var line = Console.ReadLine();
            if (line == "") break;
            Run(line);
            had_error = false;
        }
    }

    static void Run(string code)
    {
        var tokens = new Scanner(code).ScanTokens();

        foreach (var token in tokens)
        {
            Console.WriteLine(token);
        }
    }

    public static void Error(int line, string msg)
    {
        Report(line, "", msg);
    }

    static void Report(int line, string loc, string msg)
    {
        Console.WriteLine($"[line {line}] Error {loc}: {msg}");
        had_error = true;
    }
}