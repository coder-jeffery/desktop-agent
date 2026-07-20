/**
 * Simple Console Calculator using Java.
 * Usage: java CalculatorApp
 */
public class CalculatorApp {

    public static void main(String[] args) {
        System.out.println("--- Simple Java Calculator ---");
        Scanner scanner = new Scanner(System.in);

        // Get the first number
        System.out.print("请输入第一个数字 (Number 1): ");
        if (!scanner.hasNextDouble()) {
            System.err.println("错误: 输入格式不正确。请确保输入的是有效的数字。");
            return;
        }
        double num1 = scanner.nextDouble();

        // Get the operator
        System.out.print("请输入运算符 (+, -, *, /): ");
        String operator = scanner.next();

        // Get the second number
        System.out.print("请输入第二个数字 (Number 2): ");
        if (!scanner.hasNextDouble()) {
            System.err.println("错误: 输入格式不正确。请确保输入的是有效的数字。");
            return;
        }
        double num2 = scanner.nextDouble();

        double result = calculate(num1, operator, num2);

        if (result != Double.NaN) {
            System.out.println("\n计算结果: " + num1 + " " + operator + " " + num2 + " = " + result);
        } else {
            System.err.println("\n错误: 无效的运算符或操作。");
        }

        scanner.close();
    }

    /**
     * 根据提供的运算符执行相应的数学计算。
     */
    public static double calculate(double num1, String operator, double num2) {
        switch (operator) {
            case "+":
                return num1 + num2;
            case "-":
                return num1 - num2;
            case "*":
                return num1 * num2;
            case "/":
                if (num2 == 0) {
                    System.err.println("错误: 不能除以零!");
                    return Double.NaN; // 返回 NaN 表示计算失败
                }
                return num1 / num2;
            default:
                return Double.NaN; // 无效运算符
        }
    }
}

// 注意：为了使代码完整并可运行，需要导入 java.util.Scanner
import java.util.Scanner;