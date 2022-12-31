static char hello[] = "Hello world";

int main(int argc, char** argv) {
   int hello_len = 0;

   while (hello[hello_len] != '\0') {
      hello_len++;
   }

   return hello_len;
}
