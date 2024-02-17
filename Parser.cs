using static TokenType;

class Parser {
    List<Token> tokens;
    int i = 0;

    public Parser(List<Token> tokens) {
        this.tokens = tokens;
    }

    public List<Stmt> Parse() {
        List<Stmt> stmts = [];
        while (!EOF()) {
            stmts.Add(Declaration());
        }

        return stmts;
    }

    Expr Expression() {
        return Assignment();
    }

    Expr Assignment() {
        Expr expr = Equality();

        if (tokens[i].type is EQ) {
            Token prev = tokens[i];
            i++;
            Expr val = Assignment();

            if (expr is Variable variable) {
                Token ident = variable.ident;
                return new Assign(ident, val);
            }

            Lox.Error(prev, "Invalid assignment target.");
        }

        return expr;
    }

    Expr Equality() {
        Expr expression = Comparison();

        while (tokens[i].type is NOT_EQ or DBL_EQ) {
            Token op = tokens[i];
            ++i;
            Expr right = Comparison();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Comparison() {
        Expr expression = Term();

        while (tokens[i].type is GT or LT or GTE or LTE)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Term();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Term() {
        Expr expression = Factor();

        while (tokens[i].type is MINUS or PLUS)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Factor();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Factor() {
        Expr expression = Unary();

        while (tokens[i].type is ASTERISK or FWDSLASH)
        {
            Token op = tokens[i];
            ++i;
            Expr right = Unary();
            expression = new Binary(expression, op, right);
        }

        return expression!;
    }

    Expr Unary() {
        if (tokens[i].type is NOT or MINUS) {
            Token op = tokens[i];
            i++;
            Expr right = Unary();
            return new Unary(op, right);
        }

        return Primary();
    }

    Expr Primary() {
        var token = tokens[i];
        i++;
        switch (token.type) {
            case FALSE:
                return new Literal(false);
            case TRUE:
                return new Literal(true);
            case NIL:
                return new Literal(null);
            case NUMBER:
                return new Literal(token.literal);
            case STRING:
                return new Literal(token.literal);
            case PAREN_O:
                Expr expr = Expression();
                Expect(PAREN_C, "Expected ')' after expression.");
                return new Grouping(expr);
            case IDENTIFIER:
                return new Variable(token);
            default:
                throw Error(token, "Expected expression");
        }
    }

    bool EOF() {
        return i >= tokens.Count || tokens[i].type == TokenType.EOF;
    }

    void Expect(TokenType t, string message) {
        if (tokens[i].type != t) { throw Error(tokens[i], message); }
        ++i;
    }

    class ParseError : Exception { }

    ParseError Error(Token token, string message) {
        Lox.Error(token, message);
        return new ParseError();
    }

    void Sync() {
        while (!EOF()) {
            if (tokens[i - 1].type == SEMICOLON) return;
            switch (tokens[i + 1].type) {
                case CLASS:
                case FUN:
                case VAR:
                case FOR:
                case IF:
                case WHILE:
                case PRINT:
                case RETURN:
                    return;
            }
            i++;
        }
    }

    Stmt Statement() {
        var token = tokens[i];
        return token.type switch
        {
            PRINT => PrintStmt(),
            BRACKET_O => new BlockStmt(Block()),
            _ => ExprStmt(),
        };
    }

    Stmt Declaration()
    {
        try
        {
            var token = tokens[i];
            return token.type switch
            {
                VAR => VarDecl(),
                _ => Statement(),
            };
        }
        catch (ParseError e)
        {
            Sync();
            return null;
        }
    }

    Stmt PrintStmt() {
        i++;
        Expr val = Expression();
        Expect(SEMICOLON, "Expected semicolon after value of print statement");
        return new PrintStmt(val);
    }

    Stmt ExprStmt() {
        Expr val = Expression();
        Expect(SEMICOLON, "Expected semicolon after expression");
        return new ExprStmt(val);
    }

    Stmt VarDecl() {
        i++;
        Expect(IDENTIFIER, "Expected a variable name.");
        Token ident = tokens[i - 1];

        Expr init = null;

        if (tokens[i].type == EQ) {
            i++;
            init = Expression();
        }

        Expect(SEMICOLON, "Expected semicolon after var declaration");
        return new VarStmt(ident, init);
    }

    List<Stmt> Block() {
        List<Stmt> stmts = [];
        i++;

        while (!(tokens[i].type == BRACKET_C) && !EOF()) {
            stmts.Add(Declaration());
        }

        Expect(BRACKET_C, "Expected '}' to close block");
        return stmts;
    }
}