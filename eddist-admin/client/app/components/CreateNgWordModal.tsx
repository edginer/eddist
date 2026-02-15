import {
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  TextInput,
} from "flowbite-react";
import { useForm } from "react-hook-form";
import { useCreateNgWord } from "~/hooks/queries";

interface CreateNgWordModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  refetch: () => Promise<unknown>;
}

const CreateNgWordModal = ({
  setOpen,
  refetch,
  open,
}: CreateNgWordModalProps) => {
  const { register, handleSubmit } = useForm();

  const createNgWordMutation = useCreateNgWord();

  return (
    <div>
      <Modal show={open} onClose={() => setOpen(false)}>
        <ModalHeader className="border-gray-200">Create NG Word</ModalHeader>
        <ModalBody>
          <form
            onSubmit={handleSubmit((data) => {
              createNgWordMutation.mutate(
                {
                  body: {
                    name: data.name,
                    word: data.word,
                  },
                },
                {
                  onSuccess: () => {
                    setOpen(false);
                    refetch();
                  },
                },
              );
            })}
          >
            <div className="flex flex-col">
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                {...register("name", {
                  required: true,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Word</Label>
              <TextInput
                placeholder="Word..."
                required
                {...register("word", {
                  required: true,
                })}
              />
            </div>
            <Button type="submit" className="mt-4">
              Create
            </Button>
          </form>
        </ModalBody>
      </Modal>
    </div>
  );
};

export default CreateNgWordModal;
