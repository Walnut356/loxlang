using Microsoft.VisualBasic.FileIO;
using static Utils;

class Lox
{
    static bool had_error = false;

    static void Main(string[] args)
    {
        Console.WriteLine(IsDigit('.'));
        switch (args.Length)
        {
            case 0:
                runPrompt();
                break;
            case 1:
                runFile(args[0]);
                break;
            default:
                Console.WriteLine("Requires 0 or 1 args");
                break;
        }
    }

    static void runFile(String path)
    {
        string stream = File.ReadAllText(path);
        run(stream);
        if (had_error) throw new MalformedLineException();
    }

    static void runPrompt()
    {
        while (true)
        {
            Console.Write("> ");
            var line = Console.ReadLine();
            if (line == "") break;
            run(line);
            had_error = false;
        }
    }

    static void run(string code)
    {
        var tokens = new Scanner(code).ScanTokens();

        foreach (var token in tokens)
        {
            Console.WriteLine(token);
        }
    }

    public static void error(int line, string msg)
    {
        report(line, "", msg);
    }

    static void report(int line, string loc, string msg)
    {
        Console.WriteLine($"[line {line}] Error {loc}: {msg}");
        had_error = true;
    }
}