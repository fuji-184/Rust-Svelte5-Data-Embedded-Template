<script>
    import { onMount } from 'svelte';

    onMount(async () => {
        try {
            const response = await fetch('http://localhost:8000/api');
            const data = await response.json();
            // console.log(data);
        } catch (error) {
            console.error('Error fetching users:', error);
        }
    });
    

    console.log(data2)
</script>
